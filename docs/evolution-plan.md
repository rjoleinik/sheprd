# Change history for a branch before it merges

Sapling and jj keep every version of a commit: an amend or a rebase makes a
new commit and records which one it replaced (Mercurial's obsolescence
markers). Git keeps none of that, so when several agents in Sheprd tabs
rewrite the same branch, nobody can see what a commit looked like before the
last agent touched it. Sheprd keeps that history for a branch's own commits
until the branch is merged, locally and Git-compatibly, and drops it after.

## Shape

- **All tabs share the branch's one checkout.** Git checks a branch out in
  one worktree only, and a hidden worktree per tab would need a landing step
  users would not understand.
- **Nothing hooks into Git or changes a commit.** Each time an agent starts
  or finishes a turn, the checkpoint worker that snapshots the checkout also
  lists the branch's commits since it left the default branch (`origin/HEAD`,
  else `main` or `master`) and compares them with those it saw last
  (`sheprd_vcs::evolution`).
- **A replacement is recognized only on evidence:** the same `Change-Id`
  trailer, then the same `git patch-id --stable` (a rebase or reword), then
  the same subject in the same place (an amend). Anything else is recorded as
  dropped, never guessed. A turn's end credits what changed to its agent; a
  turn's start credits it to no one (made outside a turn).
- **Storage is one hidden ref per branch**, `refs/herdr-gpui/evolution/<hash
  of the branch name>`, beside checkpoints. It points at a commit whose tree
  holds the history file and whose parents are the replaced commits, so
  `git gc` keeps them. Writes move the ref only from the value read; a write
  that loses reads again. At most 500 markers per branch are kept.
- **It ends with the branch.** When the branch has no commits of its own left
  (merged), its ref is deleted; refs of deleted branches are swept at the
  next write. Nothing is pushed.

## Reading it

`sheprd change log [COMMIT]` lists each commit of the branch's own with its
earlier versions, how each was recognized, whose turn it was, and when;
changes since the last turn show as not recorded yet. `sheprd change diff
OLD [NEW]` shows how one version's patch differs from another's, or from
what it became, as `git range-diff`. Both run without the app, on the
current directory's checkout (`docs/cli.md`).

In the app, a workspace's Checkpoints dialog lists the same under Commit
versions: each rewritten commit with its earlier versions, the commits
dropped, and the command that compares two.

## Change-Id

`sheprd change id` prints a fresh Gerrit-style `Change-Id`, and the managed
skill asks agents to commit with `--trailer "Change-Id: $(sheprd change
id)"`. Amends and rebases keep the trailer, so a commit is followed even
when its patch and subject both change. Sheprd does not install a hook or
rewrite commits to add one.

## Stacked branches

Git rebases a commit's descendants on its own branch, so the commits left on
an old version are on other branches built on this one. `sheprd change
stale` finds the local branches whose commits include a replaced version and
prints the `git rebase --onto NEW OLD BRANCH` that carries each to the
current version, or says the version was dropped. It moves nothing: running
the rebase, and resolving its conflicts, stays with the agent or the user.

## Next

1. Folding each turn's edits into the tab's own change, through a temporary
   index, so a turn becomes a commit without the agent committing. This
   writes to the branch, so it would be an explicit setting, off by default.
2. Crediting edits by hunk from `sheprd-acp`'s tool calls when two agents'
   turns overlap.
3. Reading jj's own operation log where a repository already uses jj.
