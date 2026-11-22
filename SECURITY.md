# Security

Open Rebellion is a single-player game. It runs in the browser as a static
page with WebAssembly, or as a native program, and it has no accounts and no
game server. It does read files a player may get from someone else: save
games, settings, and mods for the native build.

## What to report privately

- A save game, settings file or mod that makes the game run code, read or write
  files outside its own folders, or reach the network.
- A way past the password of a hosted preview build.
- A leaked credential or token anywhere in the repository or a release.

A save or mod that only crashes the game, or a crash without a file from
someone else, is an ordinary bug: open a public
[Crash issue](https://github.com/MaxOssowski/open-rebellion/issues/new/choose).

## How to report

Use **Report a vulnerability** on the repository's
[Security tab](https://github.com/MaxOssowski/open-rebellion/security). Please
include the steps, the version (the browser console's line
`Open Rebellion build <commit>`, or the commit of your build) and, if you
can, the file that triggers it.

Expect a first answer within a week. A confirmed problem is fixed on `main`
and the report is credited unless you would rather stay anonymous.

## Supported versions

Only the current `main` branch and the build deployed from it.
