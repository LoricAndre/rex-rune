//! Files dropped on a pane, or pasted, go to its terminal's machine through Rex, which types
//! their paths into it.
use super::workspace::Workspace;
use rex_linux::{
    connection::{Command, HostCommand},
    upload::File,
};
use rune::Tx;
use std::path::PathBuf;

impl Workspace {
    /// Upload `files` to `block` of the active session. The block is fixed now; later focus
    /// changes don't move where the paths are typed.
    pub fn upload(&mut self, block: &str, files: Vec<File>) {
        let Some(session) = self.model().selected.clone() else {
            return;
        };
        let live = self
            .terminals
            .get(&self.term_key(block))
            .is_some_and(|t| t.status().live);
        if !live {
            self.inform("Files can be uploaded only to a live terminal.");
            return;
        }
        self.send(
            self.active,
            HostCommand::Upload {
                session,
                block: block.to_owned(),
                files,
            },
        );
    }

    pub fn drop_files(&mut self, block: &str, paths: &[PathBuf]) {
        if self.panel.is_some() || paths.is_empty() {
            return;
        }
        if self.focused_block().as_deref() != Some(block)
            && let Some(t) = self.terminals.get(&self.term_key(block))
        {
            let _ = t.connection.commands.focus();
        }
        self.upload(block, paths.iter().cloned().map(File::path).collect());
    }

    /// Ctrl+Shift+V: copied files upload, else text pastes, else an image uploads. The read
    /// answers later and goes only to the pane that asked, if it is still focused by then.
    pub fn paste_clipboard(&mut self, tx: &mut Tx) {
        let Some(block) = self.focused_block() else {
            return;
        };
        if !self.active_terminal().is_some_and(|t| t.status().live) {
            return;
        }
        let asked = self.term_key(&block);
        tx.read_clipboard(move |s: &mut Workspace, content, _| {
            let focused = s.focused_block().map(|block| s.term_key(&block));
            if focused.as_ref() != Some(&asked) || s.panel.is_some() {
                return;
            }
            if !content.files().is_empty() {
                let files = content.files().iter().cloned().map(File::path).collect();
                s.upload(&asked.1, files);
            } else if !content.text().is_empty() {
                if let Some(term) = s.terminals.get_mut(&asked) {
                    term.send(Command::Paste(content.text().to_owned()));
                }
            } else if let Some(image) = content.image() {
                s.upload(&asked.1, vec![File::bytes("clipboard.png", image.png())]);
            }
        });
    }
}
