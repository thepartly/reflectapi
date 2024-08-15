RUSTC = rustc

.PHONY: check deps clean

check: deps generated.rs
	$(RUSTC) --edition 2021 --crate-type=lib --emit=mir=/dev/null -L . --extern serde=libserde.rlib --extern serde_json=libserde_json.rlib --extern bytes=libbytes.rlib --extern http=libhttp.rlib --extern reflectapi=libreflectapi.rlib generated.rs

deps: libserde.rlib libserde_json.rlib libbytes.rlib libhttp.rlib libreflectapi.rlib

clean:
	rm -f *.rlib *.rmeta *.so

libserde.rlib: libserde_derive.rmeta libserde_derive.so
	$(RUSTC) --edition 2021 --crate-type=rlib --crate-name=serde --emit=metadata,link -L . --extern serde_derive=libserde_derive.rmeta --extern serde_derive=libserde_derive.so serde.rs

libserde_derive.rmeta libserde_derive.so: serde_derive.rs
	$(RUSTC) --edition 2021 --crate-type=proc-macro --crate-name=serde_derive --emit=metadata,link -L . serde_derive.rs

lib%.rlib: %.rs
	$(RUSTC) --edition 2021 --crate-type=rlib --crate-name=$* --emit=metadata,link -L . $*.rs


