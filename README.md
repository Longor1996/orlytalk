# O'Rly Talk

Under construction!

## Running

Once compiled/downloaded, run `orlytalk-server` and connect to the now running webapp trough a browser of your choice.

### Environment Variables

Environment variables can be easely applied locally by writing them into a [`.env`-file](https://github.com/dotenv-rs/dotenv#readme) in the current working directory or any of its parents.

Following is a table of all environment variables OrlyTalk will fetch and their defaults:

| Env-Var | Default | Description |
|---------|---------------|-------------|
| `ORLYTALK_SQLITE_FILE` | `db.sqlite` | The name/path to the SQLite file the server should use for persistence. |
| `ORLYTALK_HOST` | `0.0.0.0` | The host address the server should bind itself to. |
| `ORLYTALK_PORT` | `6991` | The port the server should bind itself to. |

## Building

**Dependencies** are as follows:

- [Git](https://git-scm.com/).
- A recent version of rust (`1.46` or greater).
  - Download/Install [rustup](https://rustup.rs/).
- A recent version of typescript (`4.0.2` or greater).
  - If you have NPM installed, just run `npm install -g typescript`.
- At least 1 GB of space on the drive you cloned the project to.
  - Rust and NPM cache *a lot* of stuff to make builds fast.

Then to build the project:

1. Open a console/terminal in the project directory.
2. Run `npm update`.
3. Run `tsc -p ./orly-server/src/www/ts/tsconfig.json`.
4. Run `cargo build`.
5. Done.

## Docker

To build and run O'Rly Talk for Docker on Linux-based systems, run the following commands:

```
docker build . -t orlytalk
docker run -p 6991:6991 orlytalk
```

You can then open [localhost:6991](http://localhost:6991) in your browser of choice.
