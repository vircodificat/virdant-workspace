# Virdant Workspace

A repository of documentation, feature plans, and integration tests.

## Submodule

This repo includes `virdant/` as a git submodule.

### After cloning

Run `git submodule update --init` to fetch and check out the submodule.

### Switching to a local checkout at a local checkout instead of GitHub**, edit `.gitmodules`:

```
[submodule "virdant"]
	path = virdant
	url = /home/you/path/to/virdant
```

Then run `git submodule sync` to apply the change locally.
