# Getting Started

This page is the shortest useful path through Tosumu.

## Build

```sh
cargo build
```

## Create a database

Unencrypted user-facing flow is still always authenticated internally through the sentinel protector.

```sh
cargo run -- init demo.tsm
```

To start with a passphrase-protected database:

```sh
cargo run -- init --encrypt demo.tsm
```

## Write something

```sh
cargo run -- put demo.tsm hello world
```

## Read it back

```sh
cargo run -- get demo.tsm hello
```

## Inspect it

Header summary:

```sh
cargo run -- inspect header demo.tsm --json
```

Integrity walk:

```sh
cargo run -- verify demo.tsm --explain
```

Interactive viewer:

```sh
cargo run -- view demo.tsm
```

## Useful next commands

```sh
cargo run -- stat demo.tsm
cargo run -- scan demo.tsm
cargo run -- dump demo.tsm
cargo run -- hex demo.tsm --page 1
```

## If you start with encryption

Add a recovery key after creating the database:

```sh
cargo run -- protector add-recovery-key demo.tsm
```

List configured protectors:

```sh
cargo run -- protector list demo.tsm
```

Rotate a passphrase protector KEK without rewriting pages:

```sh
cargo run -- rekey-kek demo.tsm --slot 0
```