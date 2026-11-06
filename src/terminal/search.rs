//! Searching a terminal's screen and scrollback with Ghostty's search, which keeps up with
//! new output and scrolls the viewport to the selected match.
use anyhow::Result;
use ghostty_vt::{Terminal, search::Search, selection::Selection, terminal::PointSpace};

/// One match on screen, from `start` to `end` inclusive, as viewport (column, row) cells.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Match {
    pub start: (u16, u16),
    pub end: (u16, u16),
}

/// What a frame shows of a search.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Results {
    /// Matches on screen.
    pub matches: Vec<Match>,
    /// The selected match, when it is on screen.
    pub selected: Option<Match>,
    /// The selected match's place among all matches, newest first.
    pub index: Option<usize>,
    pub total: usize,
}

impl Results {
    /// The inclusive columns each match covers on viewport row `y` of a grid `cols` wide, and
    /// whether it is the selected match.
    pub fn on_row(&self, y: usize, cols: usize) -> Vec<(usize, usize, bool)> {
        self.matches
            .iter()
            .filter(|m| (usize::from(m.start.1)..=usize::from(m.end.1)).contains(&y))
            .map(|m| {
                let first = if usize::from(m.start.1) == y {
                    usize::from(m.start.0)
                } else {
                    0
                };
                let last = if usize::from(m.end.1) == y {
                    usize::from(m.end.0)
                } else {
                    cols.saturating_sub(1)
                };
                (first, last, self.selected == Some(*m))
            })
            .collect()
    }
}

/// One terminal's search, if the user is searching it. The needle outlives the terminal it
/// was typed for, so a pane that reconnects to a fresh snapshot is searched again.
#[derive(Default)]
pub struct Controller {
    needle: String,
    search: Option<Search>,
}

impl Controller {
    /// Searches for `needle` (empty stops searching) in `terminal`, or in the terminal a
    /// snapshot brings later.
    pub fn set_needle(&mut self, terminal: Option<&Terminal>, needle: String) -> Result<()> {
        self.needle = needle;
        match (terminal, &mut self.search) {
            (Some(terminal), Some(search)) if !self.needle.is_empty() => {
                search.set_needle(terminal, Some(&self.needle))?;
            }
            _ => self.search = None,
        }
        match terminal {
            Some(terminal) => self.update(terminal),
            None => Ok(()),
        }
    }

    /// Selects the next older match (`older`) or the next newer one, wrapping around.
    pub fn step(&mut self, terminal: &Terminal, older: bool) -> Result<()> {
        if let Some(search) = &mut self.search {
            if older {
                search.select_next(terminal)?;
            } else {
                search.select_prev(terminal)?;
            }
        }
        Ok(())
    }

    /// Catches the search up with the terminal's output, starting it on a terminal that
    /// arrived after the needle. Until a match is selected, the newest one is.
    pub fn update(&mut self, terminal: &Terminal) -> Result<()> {
        if self.needle.is_empty() {
            return Ok(());
        }
        let search = match &mut self.search {
            Some(search) => search,
            None => {
                let mut search = Search::new(terminal)?;
                search.set_needle(terminal, Some(&self.needle))?;
                self.search.insert(search)
            }
        };
        search.run(terminal)?;
        if search.selected_index()?.is_none() && search.total_matches()? > 0 {
            search.select_next(terminal)?;
        }
        Ok(())
    }

    /// The search no longer applies to the terminal it ran on: a fresh snapshot replaced it.
    pub fn forget_terminal(&mut self) {
        self.search = None;
    }

    pub fn results(&self, terminal: &Terminal) -> Result<Option<Results>> {
        let Some(search) = &self.search else {
            return Ok(None);
        };
        let mut matches = Vec::new();
        for found in search.viewport_matches(terminal)? {
            matches.extend(on_screen(terminal, &found)?);
        }
        let selected = match search.selected_match(terminal)? {
            Some(found) => on_screen(terminal, &found)?,
            None => None,
        };
        Ok(Some(Results {
            matches,
            selected,
            index: search.selected_index()?,
            total: search.total_matches()?,
        }))
    }
}

/// A match's cells on screen, cut to the viewport; none when it is off screen.
fn on_screen(terminal: &Terminal, found: &Selection<'_>) -> Result<Option<Match>> {
    let point = |at| -> Result<Option<(u16, u16)>> {
        Ok(terminal
            .point_from_grid_ref(&at, PointSpace::Viewport)?
            .and_then(|p| Some((p.x, u16::try_from(p.y).ok()?))))
    };
    let last = (
        terminal.cols()?.saturating_sub(1),
        terminal.rows()?.saturating_sub(1),
    );
    Ok(match (point(found.start())?, point(found.end())?) {
        (None, None) => None,
        (start, end) => Some(Match {
            start: start.unwrap_or((0, 0)),
            end: end.unwrap_or(last),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_selects_the_newest_match_and_steps_through_the_rest() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 3,
            max_scrollback: 1 << 20,
        })
        .unwrap();
        terminal.vt_write(b"needle one\r\nother\r\nneedle two\r\nlast");
        let mut search = Controller::default();
        assert_eq!(search.results(&terminal).unwrap(), None);
        search.set_needle(Some(&terminal), "needle".into()).unwrap();
        let results = search.results(&terminal).unwrap().unwrap();
        assert_eq!(results.total, 2);
        assert_eq!(results.index, Some(0), "the newest match first");
        let newest = Match {
            start: (0, 1),
            end: (5, 1),
        };
        assert_eq!(results.selected, Some(newest));
        assert_eq!(results.on_row(1, 20), [(0, 5, true)]);

        // The older match is in scrollback: selecting it scrolls it into view.
        search.step(&terminal, true).unwrap();
        let results = search.results(&terminal).unwrap().unwrap();
        assert_eq!(results.index, Some(1));
        let selected = results.selected.expect("scrolled into view");
        assert_eq!((selected.start.0, selected.end.0), (0, 5));
        search.step(&terminal, false).unwrap();
        assert_eq!(search.results(&terminal).unwrap().unwrap().index, Some(0));

        // New output is searched too.
        terminal.vt_write(b"\r\nneedle three");
        search.update(&terminal).unwrap();
        assert_eq!(search.results(&terminal).unwrap().unwrap().total, 3);

        search.set_needle(Some(&terminal), String::new()).unwrap();
        assert_eq!(search.results(&terminal).unwrap(), None);
    }

    #[test]
    fn a_fresh_terminal_is_searched_for_the_same_needle() {
        let options = ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 3,
            ..Default::default()
        };
        let mut search = Controller::default();
        let first = Terminal::new(options).unwrap();
        search.set_needle(None, "abc".into()).unwrap();
        assert_eq!(search.results(&first).unwrap(), None, "no terminal yet");
        search.update(&first).unwrap();
        assert_eq!(search.results(&first).unwrap().unwrap().total, 0);
        search.forget_terminal();
        let mut second = Terminal::new(options).unwrap();
        second.vt_write(b"xx abc");
        search.update(&second).unwrap();
        let results = search.results(&second).unwrap().unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(
            results.matches,
            [Match {
                start: (3, 0),
                end: (5, 0)
            }]
        );
    }
}
