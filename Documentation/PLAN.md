
# Jellyfin compatible server.

## terms:

  - jellyfin: an open source media server with a rich API and many clients.
  - jellofin: a pun on the name 'jellyfin', used to describe this project. DO NOT CONFUSE IT WITH 'jellyfin'.

## Go server.

In the subdirectory jellyfin-go you'll find a server written in Go that implements two APIs:

- a custom 'notflix' api under /api, for lecacy clients.
- the jellyfin api.

## Rust server

The goal of this project is to port the Go code to Rust.

## Techstack.

Here we list the name of the tech, followed by the Go version of it, then the Rust version.

TECH    Go package                                        Rust crate
---------------------------------------------------------+---------------
http              github.com/gorilla/mux                    axum, axum_server
http              github.com/gorilla/handlers               axum, axum_server
image resizing    github.com/disintegration/imaging         image
time handling     github.com/djherbis/times                 chrono
sql               github.com/jmoiron/sqlx                   sqlx
sqlite            github.com/mattn/go-sqlite3               sqlx with sqlite feature
cli flags         github.com/spf13/pflag                    clap (using derives)
sha256            golang.org/x/crypto                       sha2
search            github.com/blevesearch/bleve/v2           tantivity
json              standard library                          serde_json
async             Go routines                               tokio

For the Rust crate versions, always use the latest one available.

## Project setup

collection/       scans a filesystem for movies and tv shows, builds collection
database/         sql database handlers
idhash/           hash a string to a 20-character identifier
imageresize/      image resizing
jellyfin/         jellyfin protocol handler
notflix           notflix protocol handler
server.go         main entry point

## Method

The Go code needs to be ported line by line:

- directory structure remains the same
- every .go file should have an equivalent .rs file
- function names should be the same
  * convering from camelcase to snakecase is permissable
- datastructure names should be the same
- datastructure fields should the the same
  * converting from PascalCase to snakecase is permissable, as long as 'serde' is instructed
    to serialize and deserialize using PascalCase
- every line of go code should be considered and functionality translated to Rust
- the code should be good and robust, but the style does not have to be perfect - we can refactor later
- copy comments

Converting data structures from Go to Rust:

- structs used for the API that get serialized from/to json:
  * each field usually has a column describing serialization/deserialization options
  * looks like `json:"MediaStreams,omitempty"`, meaning 'API field name' and 'can be null'
  * On the Rust struct, instruct serde to use PascalCase
  * if the API field name is not the same as the PascalCase API field name in rust, instruct serde to use the API field name for that field
  * If the go field contains 'omitempty'
    - the rust field should be Option<>
    - instruct serde to not serialize None option fields
  * The Rust struct should derive Default

When creating large datastructures, use ..StructName::default() where possible so the code remains small.

Interfaces:

The Go code defines interfaces for Movie, Show, Season, Episode. In Rust we prefer putting these in an enum 'Item'.
We'll probably also need an enum that contains references to on of these, use an enum called ItemRef for that. If
we need to work with traits, define them on the enum, not the member structs.

## Search

The most problematic code to translate is probably the search functionality which might no be translatable line
by line because 'bleve' and 'tantivity' are not the same. In that case, make sure that the functionality is the
same between the two implementations.

## Locking

In Rust we cannot access data structures concurrently to write to.

- Use Arc for shared read-only data, ArcSwap for data that needs to be updated
- Use Mutex for data that needs to be accessed read-write but where there is no contention (small critical section)
- Use RwLock for for data that is not suitable for ArcSwap

When scanning a collection for updates, always:

- make a clone of the collection
- update the clone
- when done swap the clone in using ArcSwap (preferred) or Mutex.

## Testing code changes

You can run cargo commands to install crates and test the code, but you CAN NOT and WILL NOT run code

OK:
- cargo add <package>
- cargo check

Not allowed:
- cargo build
- cargo run

