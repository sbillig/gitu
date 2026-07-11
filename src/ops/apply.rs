use super::OpTrait;
use crate::{
    Action, Res,
    app::{App, State},
    error::Error,
    git::diff::{Diff, PatchMode},
    item_data::ItemData,
    term::Term,
};
use std::{
    io::Write,
    process::{Command, Stdio},
    rc::Rc,
};

pub(crate) struct Apply;
impl OpTrait for Apply {
    fn get_action(&self, target: &ItemData) -> Option<Action> {
        let action = match target {
            ItemData::Stash { stash_ref, .. } => apply_stash(stash_ref.clone()),
            ItemData::Delta { diff, file_i } => apply_patch(
                diff.format_file_patch(*file_i),
                diff.format_apply_file_patch(*file_i),
                false,
            ),
            ItemData::Hunk {
                diff,
                file_i,
                hunk_i,
            } => apply_patch(
                diff.format_hunk_patch(*file_i, *hunk_i),
                diff.format_apply_hunk_patch(*file_i, *hunk_i),
                false,
            ),
            ItemData::HunkLine {
                diff,
                file_i,
                hunk_i,
                line_i,
                ..
            } => apply_line(diff, *file_i, *hunk_i, *line_i),
            _ => return None,
        };

        Some(action)
    }

    fn is_target_op(&self) -> bool {
        true
    }

    fn display(&self, _state: &State) -> String {
        "Apply".into()
    }
}

fn apply_stash(stash_ref: String) -> Action {
    Rc::new(move |app: &mut App, term: &mut Term| {
        let mut cmd = Command::new("git");
        cmd.args(["stash", "apply", "-q"]);
        cmd.arg(&stash_ref);

        app.close_menu();
        app.run_cmd(term, &[], cmd)
    })
}

fn apply_line(diff: &Rc<Diff>, file_i: usize, hunk_i: usize, line_i: usize) -> Action {
    let patch = diff
        .format_line_patch(file_i, hunk_i, line_i..(line_i + 1), PatchMode::Normal)
        .into_bytes();
    let patch_with_prerequisite = diff
        .format_apply_line_patch(file_i, hunk_i, line_i..(line_i + 1))
        .into_bytes();

    apply_patch_bytes(patch, patch_with_prerequisite, true)
}

fn apply_patch(patch: String, patch_with_prerequisite: String, recount: bool) -> Action {
    apply_patch_bytes(
        patch.into_bytes(),
        patch_with_prerequisite.into_bytes(),
        recount,
    )
}

fn apply_patch_bytes(patch: Vec<u8>, patch_with_prerequisite: Vec<u8>, recount: bool) -> Action {
    Rc::new(move |app: &mut App, term: &mut Term| {
        let patch = if patch_applies(app, &patch, recount)? {
            &patch
        } else {
            &patch_with_prerequisite
        };
        let mut cmd = Command::new("git");
        cmd.arg("apply");
        if recount {
            cmd.arg("--recount");
        }

        app.close_menu();
        app.run_cmd(term, patch, cmd)
    })
}

fn patch_applies(app: &App, patch: &[u8], recount: bool) -> Res<bool> {
    let mut cmd = Command::new("git");
    cmd.arg("apply");
    if recount {
        cmd.arg("--recount");
    }
    cmd.arg("--check")
        .current_dir(app.state.repo.workdir().expect("No workdir"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().map_err(Error::SpawnCmd)?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(patch)
        .map_err(Error::Term)?;

    Ok(child.wait().map_err(Error::CouldntAwaitCmd)?.success())
}
