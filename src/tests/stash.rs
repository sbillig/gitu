use super::*;
use crate::{app::App, item_data::ItemData, ops::Op};

fn snapshot_with_files(
    snapshot_name: &str,
    mut ctx: TestContext,
    keys_input: &str,
    files: &[&str],
) {
    let before = files
        .iter()
        .map(|name| (*name, fs::read_to_string(ctx.dir.join(name)).unwrap()))
        .collect::<Vec<_>>();

    let mut app = ctx.init_app();
    ctx.update(&mut app, keys(keys_input));

    let after = files
        .iter()
        .map(|name| (*name, fs::read_to_string(ctx.dir.join(name)).unwrap()))
        .collect::<Vec<_>>();

    let mut out = ctx.redact_buffer();
    out.push_str("\n\n[files before]\n");
    for (name, content) in before {
        out.push_str(&format!("--- {name} ---\n{content}"));
    }
    out.push_str("\n[files after]\n");
    for (name, content) in after {
        out.push_str(&format!("--- {name} ---\n{content}"));
    }

    insta::assert_snapshot!(snapshot_name, out);
}

fn setup(ctx: TestContext) -> TestContext {
    fs::write(ctx.dir.join("file-one"), "blahonga\n").unwrap();
    fs::write(ctx.dir.join("file-two"), "blahonga\n").unwrap();
    run(&ctx.dir, &["git", "add", "file-one"]);
    ctx
}

#[test]
pub(crate) fn stash_menu() {
    snapshot!(setup(setup_clone!()), "z");
}

#[test]
pub(crate) fn stash_prompt() {
    snapshot!(setup(setup_clone!()), "zz");
}

#[test]
pub(crate) fn stash() {
    snapshot!(setup(setup_clone!()), "zztest<enter>");
}

#[test]
pub(crate) fn stash_index_prompt() {
    snapshot!(setup(setup_clone!()), "zi");
}

#[test]
pub(crate) fn stash_index() {
    snapshot!(setup(setup_clone!()), "zitest<enter>");
}

#[test]
pub(crate) fn stash_working_tree_prompt() {
    snapshot!(setup(setup_clone!()), "zw");
}

#[test]
pub(crate) fn stash_working_tree() {
    snapshot!(setup(setup_clone!()), "zwtest<enter>");
}

#[test]
pub(crate) fn stash_working_tree_when_everything_is_staged() {
    snapshot!(setup(setup_clone!()), "jszw");
}

#[test]
pub(crate) fn stash_working_tree_when_nothing_is_staged() {
    let ctx = setup_clone!();
    fs::write(ctx.dir.join("file-one"), "blahonga\n").unwrap();
    snapshot!(ctx, "zwtest<enter>");
}

#[test]
pub(crate) fn stash_keeping_index_prompt() {
    snapshot!(setup(setup_clone!()), "zx");
}

#[test]
pub(crate) fn stash_keeping_index() {
    snapshot!(setup(setup_clone!()), "zxtest<enter>");
}

fn setup_two_stashes(ctx: TestContext) -> TestContext {
    let ctx = setup(ctx);
    run(
        &ctx.dir,
        &["git", "stash", "push", "--staged", "--message", "file-one"],
    );
    run(
        &ctx.dir,
        &[
            "git",
            "stash",
            "push",
            "--include-untracked",
            "--message",
            "file-two",
        ],
    );
    ctx
}

#[test]
pub(crate) fn stash_pop_prompt() {
    snapshot!(setup_two_stashes(setup_clone!()), "zp");
}

#[test]
pub(crate) fn stash_pop() {
    snapshot!(setup_two_stashes(setup_clone!()), "zp1<enter>");
}

#[test]
pub(crate) fn stash_pop_default() {
    snapshot!(setup_two_stashes(setup_clone!()), "zp<enter>");
}

#[test]
pub(crate) fn stash_apply_prompt() {
    snapshot!(setup_two_stashes(setup_clone!()), "za");
}

#[test]
pub(crate) fn stash_apply() {
    snapshot!(setup_two_stashes(setup_clone!()), "za1<enter>");
}

#[test]
pub(crate) fn stash_apply_default() {
    snapshot!(setup_two_stashes(setup_clone!()), "za<enter>");
}

#[test]
pub(crate) fn stash_drop_prompt() {
    snapshot!(setup_two_stashes(setup_clone!()), "zk");
}

#[test]
pub(crate) fn stash_drop() {
    snapshot!(setup_two_stashes(setup_clone!()), "zk1<enter>");
}

#[test]
pub(crate) fn stash_drop_default() {
    snapshot!(setup_two_stashes(setup_clone!()), "zk<enter>");
}

fn setup_stash_for_patch_apply(ctx: TestContext) -> TestContext {
    commit(
        &ctx.dir,
        "file1.txt",
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n",
    );
    commit(&ctx.dir, "file2.txt", "alpha\nbeta\ngamma\n");

    fs::write(ctx.dir.join("file1.txt"), "one\ntwo\ntwo-and-a-half\ntwo-and-three-quarters\nthree\nfour\nfive\nsix\nseven\neight\nnine\nTEN\n").unwrap();
    fs::write(ctx.dir.join("file2.txt"), "alpha\nbeta\ngamma\ndelta\n").unwrap();
    run(&ctx.dir, &["git", "stash", "save", "apply-stash"]);
    ctx
}

fn setup_stash_with_staged_prerequisite(ctx: TestContext) -> TestContext {
    commit(&ctx.dir, "overlap.txt", "one\ntwo\nthree\n");
    fs::write(ctx.dir.join("overlap.txt"), "one\nTWO-STAGED\nthree\n").unwrap();
    run(&ctx.dir, &["git", "add", "overlap.txt"]);
    fs::write(ctx.dir.join("overlap.txt"), "one\nTWO-UNSTAGED\nthree\n").unwrap();
    run(&ctx.dir, &["git", "stash", "save", "apply-stash"]);
    ctx
}

fn setup_stash_with_independent_staged_change(ctx: TestContext) -> TestContext {
    commit(
        &ctx.dir,
        "independent.txt",
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n",
    );
    fs::write(
        ctx.dir.join("independent.txt"),
        "ONE-STAGED\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n",
    )
    .unwrap();
    run(&ctx.dir, &["git", "add", "independent.txt"]);
    fs::write(
        ctx.dir.join("independent.txt"),
        "ONE-STAGED\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nTEN-UNSTAGED\n",
    )
    .unwrap();
    run(&ctx.dir, &["git", "stash", "save", "apply-stash"]);
    ctx
}

fn open_unstaged_stash_diff(ctx: &mut TestContext) -> App {
    let mut app = ctx.init_app();
    ctx.update(&mut app, keys("jj<enter>"));
    app.handle_op(Op::MoveNextSection, &mut ctx.term).unwrap();
    app.handle_op(Op::MoveDown, &mut ctx.term).unwrap();

    let ItemData::Delta { diff, .. } = &app.screen().get_selected_item().data else {
        panic!("expected the unstaged stash file")
    };
    assert!(diff.apply_prerequisite.is_some());

    app
}

#[test]
pub(crate) fn stash_apply_hunk_as_patch() {
    let ctx = setup_stash_for_patch_apply(setup_clone!());
    let snapshot_name = function_name!().rsplit("::").next().unwrap();
    snapshot_with_files(
        snapshot_name,
        ctx,
        "jj<enter>a",
        &["file1.txt", "file2.txt"],
    );
}

#[test]
pub(crate) fn stash_apply_file_as_patch() {
    let ctx = setup_stash_for_patch_apply(setup_clone!());
    let snapshot_name = function_name!().rsplit("::").next().unwrap();
    snapshot_with_files(
        snapshot_name,
        ctx,
        "jj<enter><alt+h>a",
        &["file1.txt", "file2.txt"],
    );
}

#[test]
pub(crate) fn stash_apply_line_as_patch() {
    let ctx = setup_stash_for_patch_apply(setup_clone!());
    let snapshot_name = function_name!().rsplit("::").next().unwrap();
    snapshot_with_files(
        snapshot_name,
        ctx,
        "jj<enter><ctrl+j>a",
        &["file1.txt", "file2.txt"],
    );
}

#[test]
pub(crate) fn stash_apply_selected() {
    let ctx = setup_stash_for_patch_apply(setup_clone!());
    let snapshot_name = function_name!().rsplit("::").next().unwrap();
    snapshot_with_files(snapshot_name, ctx, "jja", &["file1.txt", "file2.txt"]);
}

#[test]
pub(crate) fn stash_apply_unstaged_file_with_staged_prerequisite() {
    let mut ctx = setup_stash_with_staged_prerequisite(setup_clone!());
    let mut app = open_unstaged_stash_diff(&mut ctx);
    app.handle_op(Op::Apply, &mut ctx.term).unwrap();

    assert_eq!(
        fs::read_to_string(ctx.dir.join("overlap.txt")).unwrap(),
        "one\nTWO-UNSTAGED\nthree\n"
    );
}

#[test]
pub(crate) fn stash_apply_unstaged_hunk_with_staged_prerequisite() {
    let mut ctx = setup_stash_with_staged_prerequisite(setup_clone!());
    let mut app = open_unstaged_stash_diff(&mut ctx);
    app.handle_op(Op::MoveDown, &mut ctx.term).unwrap();
    app.handle_op(Op::Apply, &mut ctx.term).unwrap();

    assert_eq!(
        fs::read_to_string(ctx.dir.join("overlap.txt")).unwrap(),
        "one\nTWO-UNSTAGED\nthree\n"
    );
}

#[test]
pub(crate) fn stash_apply_unstaged_line_with_staged_prerequisite() {
    let mut ctx = setup_stash_with_staged_prerequisite(setup_clone!());
    let mut app = open_unstaged_stash_diff(&mut ctx);
    app.handle_op(Op::MoveDown, &mut ctx.term).unwrap();
    app.handle_op(Op::MoveDownLine, &mut ctx.term).unwrap();
    app.handle_op(Op::Apply, &mut ctx.term).unwrap();

    assert_eq!(
        fs::read_to_string(ctx.dir.join("overlap.txt")).unwrap(),
        "one\nthree\n"
    );
}

#[test]
pub(crate) fn stash_apply_unstaged_hunk_without_unrelated_staged_changes() {
    let mut ctx = setup_stash_with_independent_staged_change(setup_clone!());
    let mut app = open_unstaged_stash_diff(&mut ctx);
    app.handle_op(Op::MoveDown, &mut ctx.term).unwrap();
    app.handle_op(Op::Apply, &mut ctx.term).unwrap();

    assert_eq!(
        fs::read_to_string(ctx.dir.join("independent.txt")).unwrap(),
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nTEN-UNSTAGED\n"
    );
}
