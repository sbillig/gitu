use crate::gitu_diff::{FileDiff, FilePath};
use std::{ops::Range, rc::Rc};

#[derive(Debug, Clone)]
pub(crate) struct Diff {
    pub text: String,
    pub diff_type: DiffType,
    pub file_diffs: Vec<FileDiff>,
    pub apply_prerequisite: Option<Rc<Diff>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffType {
    WorkdirToIndex, // i.e. Unstaged
    IndexToTree,    // i.e. Staged
    TreeToTree,
    Stash,
}

impl DiffType {
    pub(crate) fn is_stash(self) -> bool {
        matches!(self, Self::Stash)
    }
}

#[derive(Debug)]
pub(crate) enum PatchMode {
    Normal,
    Reverse,
}

impl Diff {
    pub(crate) fn mask_old_hunk(&self, file_i: usize, hunk_i: usize) -> String {
        let content = &self.text[self.file_diffs[file_i].hunks[hunk_i].content.range.clone()];
        mask_hunk_content(content, '-', '+')
    }

    pub(crate) fn mask_new_hunk(&self, file_i: usize, hunk_i: usize) -> String {
        let content = &self.text[self.file_diffs[file_i].hunks[hunk_i].content.range.clone()];
        mask_hunk_content(content, '+', '-')
    }

    pub(crate) fn format_file_patch(&self, file_i: usize) -> String {
        let mut patch = String::new();
        patch.push_str(self.file_diff_header(file_i));
        for hunk in &self.file_diffs[file_i].hunks {
            patch.push_str(&self.text[hunk.range.clone()]);
        }
        patch
    }

    pub(crate) fn format_apply_file_patch(&self, file_i: usize) -> String {
        self.with_prerequisite(file_i, self.format_file_patch(file_i))
    }

    pub(crate) fn format_hunk_patch(&self, file_i: usize, hunk_i: usize) -> String {
        let mut patch = String::new();
        patch.push_str(self.file_diff_header(file_i));
        patch.push_str(self.hunk(file_i, hunk_i));
        patch
    }

    pub(crate) fn format_apply_hunk_patch(&self, file_i: usize, hunk_i: usize) -> String {
        self.with_prerequisite(file_i, self.format_hunk_patch(file_i, hunk_i))
    }

    pub(crate) fn file_diff_header(&self, file_i: usize) -> &str {
        &self.text[self.file_diffs[file_i].header.range.clone()]
    }

    pub(crate) fn hunk(&self, file_i: usize, hunk_i: usize) -> &str {
        &self.text[self.file_diffs[file_i].hunks[hunk_i].range.clone()]
    }

    pub(crate) fn hunk_content(&self, file_index: usize, hunk_index: usize) -> &str {
        &self.text[self.file_diffs[file_index].hunks[hunk_index]
            .content
            .range
            .clone()]
    }

    pub(crate) fn format_line_patch(
        &self,
        file_i: usize,
        hunk_i: usize,
        line_range: Range<usize>,
        mode: PatchMode,
    ) -> String {
        let hunk = &self.file_diffs[file_i].hunks[hunk_i];
        let file_header = &self.text[self.file_diffs[file_i].header.range.clone()];
        let hunk_header = &self.text[hunk.header.range.clone()];
        let hunk_content = &self.text[hunk.content.range.clone()];

        let modified_content = hunk_content
            .split_inclusive('\n')
            .enumerate()
            .filter_map(|(i, line)| {
                let add = match mode {
                    PatchMode::Normal => '+',
                    PatchMode::Reverse => '-',
                };

                let remove = match mode {
                    PatchMode::Normal => '-',
                    PatchMode::Reverse => '+',
                };

                if line_range.contains(&i) {
                    Some(line.to_string())
                } else if line.starts_with(add) {
                    None
                } else if let Some(stripped) = line.strip_prefix(remove) {
                    Some(format!(" {stripped}"))
                } else {
                    Some(line.to_string())
                }
            })
            .collect::<String>();

        format!("{file_header}{hunk_header}{modified_content}")
    }

    pub(crate) fn format_apply_line_patch(
        &self,
        file_i: usize,
        hunk_i: usize,
        line_range: Range<usize>,
    ) -> String {
        self.with_prerequisite(
            file_i,
            self.format_line_patch(file_i, hunk_i, line_range, PatchMode::Normal),
        )
    }

    pub(crate) fn file_line_of_first_diff(&self, file_i: usize, hunk_i: usize) -> usize {
        let hunk = &self.file_diffs[file_i].hunks[hunk_i];
        let line = hunk.header.new_line_start as usize;

        let hunk_content = &self.text[hunk.content.range.clone()];
        for (i, content_line) in hunk_content.lines().enumerate() {
            if content_line.starts_with('+') || content_line.starts_with('-') {
                return line + i;
            }
        }
        line
    }

    fn with_prerequisite(&self, file_i: usize, patch: String) -> String {
        format!(
            "{}{}",
            self.prerequisite_file_patch(file_i).unwrap_or_default(),
            patch
        )
    }

    fn prerequisite_file_patch(&self, file_i: usize) -> Option<String> {
        let prerequisite = self.apply_prerequisite.as_ref()?;
        let paths = self.file_paths(file_i);
        let prerequisite_file_i = prerequisite.file_diffs.iter().position(|file_diff| {
            [
                normalized_file_path(&file_diff.header.old_file, &prerequisite.text),
                normalized_file_path(&file_diff.header.new_file, &prerequisite.text),
            ]
            .into_iter()
            .flatten()
            .any(|path| paths.contains(&path))
        })?;

        Some(prerequisite.format_file_patch(prerequisite_file_i))
    }

    fn file_paths(&self, file_i: usize) -> Vec<String> {
        let header = &self.file_diffs[file_i].header;
        [
            normalized_file_path(&header.old_file, &self.text),
            normalized_file_path(&header.new_file, &self.text),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

fn normalized_file_path(file: &FilePath, text: &str) -> Option<String> {
    let path = &text[file.range.clone()];
    (path != "/dev/null").then(|| path.to_string())
}

fn mask_hunk_content(content: &str, keep: char, mask: char) -> String {
    let mut result = String::new();

    content.split_inclusive('\n').for_each(|line| {
        if line.starts_with(mask) {
            if line.ends_with("\r\n") {
                for _ in 0..(line.len() - 2) {
                    result.push(' ');
                }
                result.push_str("\r\n");
            } else if line.ends_with('\n') {
                for _ in 0..(line.len() - 1) {
                    result.push(' ');
                }
                result.push('\n');
            }
        } else if line.starts_with(keep) {
            result.push(' ');
            result.push_str(&line[1..]);
        } else if !line.starts_with('\\') {
            result.push_str(line);
        }
    });

    result
}
