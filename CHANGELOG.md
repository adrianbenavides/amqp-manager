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
