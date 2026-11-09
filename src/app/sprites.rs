//! Box-drawing (U+2500–U+257F) and block element (U+2580–U+259F) characters, drawn as
//! shapes in whole device pixels, as Ghostty's sprites are, rather than from the font. Font
//! glyphs rarely fill a cell exactly, so their lines and blocks leave seams, gaps or overlaps
//! between neighbouring cells; these meet their neighbours' edges whatever the font.
use rune::{Color, El, LineCap, Path, Shape, canvas, el};

/// How heavy a box-drawing line from a cell's edge to its centre is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    None,
    Light,
    Heavy,
    Double,
}

/// U+2500–U+257F as Ghostty draws them: the line from each edge to the centre as
/// `up right down left` (`.` none, `l` light, `h` heavy, `d` double). `....` marks the
/// dashed lines, rounded corners and diagonals, which `Sprite::of` names itself.
const BOX_LINES: &str = "\
.l.l .h.h l.l. h.h. .... .... .... .... .... .... .... .... .ll. .hl. .lh. .hh. \
..ll ..lh ..hl ..hh ll.. lh.. hl.. hh.. l..l l..h h..l h..h lll. lhl. hll. llh. \
hlh. hhl. lhh. hhh. l.ll l.lh h.ll l.hl h.hl h.lh l.hh h.hh .lll .llh .hll .hlh \
.lhl .lhh .hhl .hhh ll.l ll.h lh.l lh.h hl.l hl.h hh.l hh.h llll lllh lhll lhlh \
hlll llhl hlhl hllh hhll llhh lhhl hhlh lhhh hlhh hhhl hhhh .... .... .... .... \
.d.d d.d. .dl. .ld. .dd. ..ld ..dl ..dd ld.. dl.. dd.. l..d d..l d..d ldl. dld. \
ddd. l.ld d.dl d.dd .dld .ldl .ddd ld.d dl.l dd.d ldld dldl dddd .... .... .... \
.... .... .... .... ...l l... .l.. ..l. ...h h... .h.. ..h. .h.l l.h. .l.h h.l. ";

/// The gap Ghostty leaves between dashes, at most: four pixels (but no less than a light
/// line), or one light or heavy line.
#[derive(Clone, Copy)]
pub enum DashGap {
    Four,
    Light,
    Heavy,
}

/// A character drawn as a sprite.
#[derive(Clone, Copy)]
pub enum Sprite {
    /// Box-drawing lines: up, right, down and left.
    Lines([Weight; 4]),
    Dashes {
        vertical: bool,
        heavy: bool,
        count: u8,
        gap: DashGap,
    },
    /// A rounded corner joining the lines towards `dx` (1 right, -1 left) and `dy` (1 down,
    /// -1 up).
    Arc {
        dx: f32,
        dy: f32,
    },
    Diagonal {
        rising: bool,
        falling: bool,
    },
    /// A block element.
    Block(char),
}

impl Sprite {
    pub fn of(glyph: char) -> Option<Self> {
        if ('\u{2580}'..='\u{259f}').contains(&glyph) {
            return Some(Self::Block(glyph));
        }
        let index = (glyph as u32).checked_sub(0x2500).filter(|&i| i < 0x80)? as usize;
        let dashes = |vertical, heavy, count, gap| Self::Dashes {
            vertical,
            heavy,
            count,
            gap,
        };
        Some(match glyph {
            '┄' => dashes(false, false, 3, DashGap::Four),
            '┅' => dashes(false, true, 3, DashGap::Four),
            '┆' => dashes(true, false, 3, DashGap::Four),
            '┇' => dashes(true, true, 3, DashGap::Four),
            '┈' => dashes(false, false, 4, DashGap::Four),
            '┉' => dashes(false, true, 4, DashGap::Four),
            '┊' => dashes(true, false, 4, DashGap::Four),
            '┋' => dashes(true, true, 4, DashGap::Four),
            '╌' => dashes(false, false, 2, DashGap::Light),
            '╍' => dashes(false, true, 2, DashGap::Heavy),
            '╎' => dashes(true, false, 2, DashGap::Heavy),
            '╏' => dashes(true, true, 2, DashGap::Heavy),
            '╭' => Self::Arc { dx: 1., dy: 1. },
            '╮' => Self::Arc { dx: -1., dy: 1. },
            '╯' => Self::Arc { dx: -1., dy: -1. },
            '╰' => Self::Arc { dx: 1., dy: -1. },
            '╱' => Self::Diagonal {
                rising: true,
                falling: false,
            },
            '╲' => Self::Diagonal {
                rising: false,
                falling: true,
            },
            '╳' => Self::Diagonal {
                rising: true,
                falling: true,
            },
            _ => {
                let weight = |b| match b {
                    b'l' => Weight::Light,
                    b'h' => Weight::Heavy,
                    b'd' => Weight::Double,
                    _ => Weight::None,
                };
                let code = &BOX_LINES.as_bytes()[index * 5..index * 5 + 4];
                Self::Lines([
                    weight(code[0]),
                    weight(code[1]),
                    weight(code[2]),
                    weight(code[3]),
                ])
            }
        })
    }

    /// Adds the sprite to `row` for a cell `x` points from the row's start, `size` points
    /// large, drawn at `px` device pixels per point.
    pub fn draw(self, row: El, x: f32, size: (f32, f32), px: f32, color: Color) -> El {
        let cell = SpriteCell {
            x,
            w: size.0 * px,
            h: size.1 * px,
            px,
            light: px.round().max(1.),
            color,
        };
        match self {
            Self::Lines(lines) => cell.lines(row, lines),
            Self::Dashes {
                vertical,
                heavy,
                count,
                gap,
            } => cell.dashes(row, vertical, heavy, count, gap),
            Self::Arc { dx, dy } => cell.arc(row, dx, dy),
            Self::Diagonal { rising, falling } => cell.diagonal(row, rising, falling),
            Self::Block(glyph) => cell.block(row, glyph),
        }
    }
}

/// One cell's sprite, laid out in device pixels from its top-left corner: lines are whole
/// pixels thick and start on whole pixels, so pixel snapping keeps every thickness and a line
/// meets its neighbour's with no seam or overlap.
struct SpriteCell {
    /// The cell's left edge in the row, in points.
    x: f32,
    /// The cell's size in device pixels.
    w: f32,
    h: f32,
    px: f32,
    /// A light line's thickness: one point, in whole device pixels.
    light: f32,
    color: Color,
}

impl SpriteCell {
    /// Where a `thick` line centred across `extent` starts.
    fn middle(extent: f32, thick: f32) -> f32 {
        ((extent - thick) / 2.).floor().max(0.)
    }

    /// A rectangle between device-pixel edges, in `color`.
    fn fill(&self, row: El, [x0, y0, x1, y1]: [f32; 4], color: Color) -> El {
        if x1 <= x0 || y1 <= y0 {
            return row;
        }
        let px = self.px;
        row.child(
            el().absolute(self.x + x0 / px, y0 / px)
                .size((x1 - x0) / px, (y1 - y0) / px)
                .fill(color),
        )
    }

    fn rect(&self, row: El, x0: f32, y0: f32, x1: f32, y1: f32) -> El {
        self.fill(row, [x0, y0, x1, y1], self.color)
    }

    /// Ghostty's `linesChar`: each line runs from its edge to where it meets the others.
    fn lines(&self, mut row: El, [up, right, down, left]: [Weight; 4]) -> El {
        use Weight::{Double, Heavy, Light, None};
        let (w, h, light) = (self.w, self.h, self.light);
        let heavy = 2. * light;
        let h_light_top = Self::middle(h, light);
        let h_light_bottom = h_light_top + light;
        let h_heavy_top = Self::middle(h, heavy);
        let h_heavy_bottom = h_heavy_top + heavy;
        let h_double_top = (h_light_top - light).max(0.);
        let h_double_bottom = h_light_bottom + light;
        let v_light_left = Self::middle(w, light);
        let v_light_right = v_light_left + light;
        let v_heavy_left = Self::middle(w, heavy);
        let v_heavy_right = v_heavy_left + heavy;
        let v_double_left = (v_light_left - light).max(0.);
        let v_double_right = v_light_right + light;

        let up_bottom = if left == Heavy || right == Heavy {
            h_heavy_bottom
        } else if left != right || down == up {
            if left == Double || right == Double {
                h_double_bottom
            } else {
                h_light_bottom
            }
        } else if left == None && right == None {
            h_light_bottom
        } else {
            h_light_top
        };
        let down_top = if left == Heavy || right == Heavy {
            h_heavy_top
        } else if left != right || up == down {
            if left == Double || right == Double {
                h_double_top
            } else {
                h_light_top
            }
        } else if left == None && right == None {
            h_light_top
        } else {
            h_light_bottom
        };
        let left_right = if up == Heavy || down == Heavy {
            v_heavy_right
        } else if up != down || left == right {
            if up == Double || down == Double {
                v_double_right
            } else {
                v_light_right
            }
        } else if up == None && down == None {
            v_light_right
        } else {
            v_light_left
        };
        let right_left = if up == Heavy || down == Heavy {
            v_heavy_left
        } else if up != down || right == left {
            if up == Double || down == Double {
                v_double_left
            } else {
                v_light_left
            }
        } else if up == None && down == None {
            v_light_left
        } else {
            v_light_right
        };

        match up {
            None => {}
            Light => row = self.rect(row, v_light_left, 0., v_light_right, up_bottom),
            Heavy => row = self.rect(row, v_heavy_left, 0., v_heavy_right, up_bottom),
            Double => {
                let left_bottom = if left == Double {
                    h_light_top
                } else {
                    up_bottom
                };
                let right_bottom = if right == Double {
                    h_light_top
                } else {
                    up_bottom
                };
                row = self.rect(row, v_double_left, 0., v_light_left, left_bottom);
                row = self.rect(row, v_light_right, 0., v_double_right, right_bottom);
            }
        }
        match right {
            None => {}
            Light => row = self.rect(row, right_left, h_light_top, w, h_light_bottom),
            Heavy => row = self.rect(row, right_left, h_heavy_top, w, h_heavy_bottom),
            Double => {
                let top_left = if up == Double {
                    v_light_right
                } else {
                    right_left
                };
                let bottom_left = if down == Double {
                    v_light_right
                } else {
                    right_left
                };
                row = self.rect(row, top_left, h_double_top, w, h_light_top);
                row = self.rect(row, bottom_left, h_light_bottom, w, h_double_bottom);
            }
        }
        match down {
            None => {}
            Light => row = self.rect(row, v_light_left, down_top, v_light_right, h),
            Heavy => row = self.rect(row, v_heavy_left, down_top, v_heavy_right, h),
            Double => {
                let left_top = if left == Double {
                    h_light_bottom
                } else {
                    down_top
                };
                let right_top = if right == Double {
                    h_light_bottom
                } else {
                    down_top
                };
                row = self.rect(row, v_double_left, left_top, v_light_left, h);
                row = self.rect(row, v_light_right, right_top, v_double_right, h);
            }
        }
        match left {
            None => {}
            Light => row = self.rect(row, 0., h_light_top, left_right, h_light_bottom),
            Heavy => row = self.rect(row, 0., h_heavy_top, left_right, h_heavy_bottom),
            Double => {
                let top_right = if up == Double {
                    v_light_left
                } else {
                    left_right
                };
                let bottom_right = if down == Double {
                    v_light_left
                } else {
                    left_right
                };
                row = self.rect(row, 0., h_double_top, top_right, h_light_top);
                row = self.rect(row, 0., h_light_bottom, bottom_right, h_double_bottom);
            }
        }
        row
    }

    /// Ghostty's dashed lines: `count` dashes per cell that tile into an even line, with
    /// half gaps at both ends of a horizontal cell and a whole gap at the foot of a
    /// vertical one.
    fn dashes(&self, mut row: El, vertical: bool, heavy: bool, count: u8, gap: DashGap) -> El {
        let light = self.light;
        let thick = if heavy { 2. * light } else { light };
        let desired = match gap {
            DashGap::Four => light.max(4.),
            DashGap::Light => light,
            DashGap::Heavy => 2. * light,
        };
        let n = f32::from(count);
        let (along, across) = if vertical {
            (self.h, self.w)
        } else {
            (self.w, self.h)
        };
        let side = Self::middle(across, thick);
        let span = along.floor();
        if span < 2. * n {
            return if vertical {
                let x = Self::middle(self.w, light);
                self.rect(row, x, 0., x + light, self.h)
            } else {
                let y = Self::middle(self.h, light);
                self.rect(row, 0., y, self.w, y + light)
            };
        }
        let gap = desired.min((span / (2. * n)).floor());
        let total = span - n * gap;
        let dash = (total / n).floor();
        let mut extra = total - dash * n;
        let mut at = if vertical { 0. } else { (gap / 2.).floor() };
        for _ in 0..count {
            let mut end = at + dash;
            // Leftover pixels lengthen dashes, which shows less than uneven gaps.
            if extra > 0. {
                extra -= 1.;
                end += 1.;
            }
            row = if vertical {
                self.rect(row, side, at, side + thick, end)
            } else {
                self.rect(row, at, side, end, side + thick)
            };
            at = end + gap;
        }
        row
    }

    /// Ghostty's rounded corner: a curve from the middle of one edge to the middle of the
    /// other. Its straight ends are drawn again as whole-pixel lines over the curve, so they
    /// meet the neighbouring cells' lines exactly.
    fn arc(&self, mut row: El, dx: f32, dy: f32) -> El {
        let (w, h, light, px) = (self.w, self.h, self.light, self.px);
        let left = Self::middle(w, light);
        let top = Self::middle(h, light);
        let (cx, cy) = (left + light / 2., top + light / 2.);
        let r = (w.min(h) / 2.)
            .min(if dx > 0. { w - cx } else { cx })
            .min(if dy > 0. { h - cy } else { cy });
        // How far the inner control points sit from the corner, as a share of `r`.
        let s = 0.25;
        let at = |x: f32, y: f32| [x / px, y / px];
        let path = Path::new().move_to(at(cx, cy + dy * r)).cubic_to(
            at(cx, cy + dy * s * r),
            at(cx + dx * s * r, cy),
            at(cx + dx * r, cy),
        );
        row = row.child(
            canvas(Shape::new().stroke_path(&path, light / px, LineCap::Butt, self.color))
                .absolute(self.x, 0.)
                .size(w / px, h / px),
        );
        // Within a fifth of `r` of its ends the curve is less than 1% of `r` off straight.
        let (x, y) = ((cx + dx * 0.8 * r).round(), (cy + dy * 0.8 * r).round());
        row = if dy > 0. {
            self.rect(row, left, y, left + light, h)
        } else {
            self.rect(row, left, 0., left + light, y)
        };
        if dx > 0. {
            self.rect(row, x, top, w, top + light)
        } else {
            self.rect(row, 0., top, x, top + light)
        }
    }

    /// Corner-to-corner lines that overshoot the cell slightly along their own slope, so
    /// diagonals in neighbouring cells join.
    fn diagonal(&self, row: El, rising: bool, falling: bool) -> El {
        let (w, h, px) = (self.w, self.h, self.px);
        let (sx, sy) = ((w / h).min(1.) * 0.5, (h / w).min(1.) * 0.5);
        let at = |x: f32, y: f32| [x / px, y / px];
        let width = self.light / px;
        let mut shape = Shape::new();
        if rising {
            shape = shape.line(
                at(w + sx, -sy),
                at(-sx, h + sy),
                width,
                LineCap::Butt,
                self.color,
            );
        }
        if falling {
            shape = shape.line(
                at(-sx, -sy),
                at(w + sx, h + sy),
                width,
                LineCap::Butt,
                self.color,
            );
        }
        row.child(canvas(shape).absolute(self.x, 0.).size(w / px, h / px))
    }

    /// Block elements: fills with edges at eighths and halves of the cell, rounded to whole
    /// device pixels, and the three shades as translucent fills.
    fn block(&self, row: El, glyph: char) -> El {
        let (w, h) = (self.w, self.h);
        let x = |share: f32| (w * share).round();
        let y = |share: f32| (h * share).round();
        // Lower and left blocks count eighths up from U+2580 and down to U+2590.
        let eighths = |from: u32, to: u32| (to - from) as f32 / 8.;
        match glyph {
            '▀' => self.rect(row, 0., 0., w, y(0.5)),
            '▁'..='▇' => self.rect(row, 0., y(1. - eighths(0x2580, glyph as u32)), w, h),
            '█' => self.rect(row, 0., 0., w, h),
            '▉'..='▏' => self.rect(row, 0., 0., x(eighths(glyph as u32, 0x2590)), h),
            '▐' => self.rect(row, x(0.5), 0., w, h),
            '░' => self.fill(row, [0., 0., w, h], self.color.alpha(0.25)),
            '▒' => self.fill(row, [0., 0., w, h], self.color.alpha(0.5)),
            '▓' => self.fill(row, [0., 0., w, h], self.color.alpha(0.75)),
            '▔' => self.rect(row, 0., 0., w, y(1. / 8.)),
            '▕' => self.rect(row, x(7. / 8.), 0., w, h),
            quadrant => {
                // Upper left, upper right, lower left and lower right.
                let bits = match quadrant {
                    '▖' => 0b0100,
                    '▗' => 0b1000,
                    '▘' => 0b0001,
                    '▙' => 0b1101,
                    '▚' => 0b1001,
                    '▛' => 0b0111,
                    '▜' => 0b1011,
                    '▝' => 0b0010,
                    '▞' => 0b0110,
                    '▟' => 0b1110,
                    _ => 0,
                };
                let (mx, my) = (x(0.5), y(0.5));
                [
                    [0., 0., mx, my],
                    [mx, 0., w, my],
                    [0., my, mx, h],
                    [mx, my, w, h],
                ]
                .into_iter()
                .enumerate()
                .filter(|(bit, _)| bits & (1 << bit) != 0)
                .fold(row, |row, (_, edges)| self.fill(row, edges, self.color))
            }
        }
    }
}
