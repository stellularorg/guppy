# 🐠 guppy

[Guppy](https://guppy.stellular.net) is a simple user management system.

> Guppy is a relocation and rewrite of [Bundlrs](https://code.stellular.org/stellular/bundlrs) user accounts

Guppy only supports the board-orientated markup features of Bundlrs.

**Guppy is designed to be able to interface with the same database as a Bundlrs instance!** It will use existing users from the `Users` table and existing posts from the `Logs` table.

## Configuration

Guppy requires the following environment variables to integrate with the other required services:

* `BUNDLRS_ROOT` - root address of the bundlrs server

The rest of the configuration (for databases) can be found in the [Bundlrs README](https://code.stellular.org/stellular/bundlrs/src/branch/master/README.md).

You can require that an "invite code" be provided when registering an account by passing the `INVITE_CODES` environment variable:

```ini
# allow invite codes "abcd" and "12345"
INVITE_CODES="abcd,12345"
```

Passing this variable will require an invite code when registering any account.
