## 1.3.0 (2021-10-23)

**Features**

* Add `deadpool` pool connection, which somehow reverts the changes from version `1.1.0`.  
* Refactor the `AmqpResult` type to include errors from both `lapin` and `deadpool`.

**Breaking**

* Including an async pool connection implies using an async runtime, so as of this release, this library will include
a couple of features to specify the runtime you want to use (tokio or async_std).

## 1.2.0 (2021-05-21)

**Features**

* Refactor `AmqpManager` to add the necessary fields to create connections more easily

## 1.1.0 (2021-05-21)

**Features**

* Remove `mobc` dependency to avoid depending on `tokio` and forcing users to use lapin with the tokio runtime only
* As a result of the previous change, the `AmqpManager` is now stateless and the `new` method has been removed 

## 1.0.0 (2021-03-11)

**Features**

* Support for tokio 1

## 0.6.x (2021-03-11)

**Features**

* Remove `Clone` trait from `AmqpSession`
* Replace usage of the `Payload` struct by a raw bytes vector
* Add `routing_key` parameter to exchange related operations


## 0.3.x (2021-01-29)

**Features**

* Use explicit lifetimes on `ops` structs to avoid unnecessary allocations

## 0.1.0 (2020-09-26)

**Features**

* Initial release
* Support for tokio 0.2
