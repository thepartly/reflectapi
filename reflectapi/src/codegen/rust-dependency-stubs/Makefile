RUSTC = rustc

.PHONY: check deps clean

check: libreflectapi.rlib \
            libserde.rlib \
            libchrono.rlib \
            libchrono_tz.rlib \
            libserde_json.rlib \
            libindexmap.rlib \
            generated.rs
	$(RUSTC) --edition 2021 --crate-type=lib --emit=mir=/dev/null -L . \
		--extern reflectapi=libreflectapi.rlib \
		--extern serde=libserde.rlib \
		--extern serde_json=libserde_json.rlib \
		--extern indexmap=libindexmap.rlib \
		--extern chrono=libchrono.rlib \
		--extern chrono_tz=libchrono_tz.rlib \
		--extern url=liburl.rlib \
		generated.rs

clean:
	rm -f *.rlib *.rmeta *.so

# Make sure to edit `typecheck` in `codegen/rust.rs` to copy all the files into the right place
libreflectapi.rlib: libserde.rlib \
                    libserde_json.rlib \
                    libbytes.rlib \
                    libhttp.rlib \
                    liburl.rlib \
                    reflectapi.rs
	$(RUSTC) --edition 2021 --crate-type=rlib --crate-name=reflectapi --emit=metadata,link -L . \
		--extern serde=libserde.rlib \
		--extern serde_json=libserde_json.rlib \
		--extern bytes=libbytes.rlib \
		--extern http=libhttp.rlib \
		--extern url=liburl.rlib \
		reflectapi.rs

libserde.rlib: libserde_derive.rmeta libserde_derive.so
	$(RUSTC) --edition 2021 --crate-type=rlib --crate-name=serde --emit=metadata,link -L . --extern serde_derive=libserde_derive.rmeta --extern serde_derive=libserde_derive.so serde.rs

libserde_derive.rmeta libserde_derive.so: serde_derive.rs
	$(RUSTC) --edition 2021 --crate-type=proc-macro --crate-name=serde_derive --emit=metadata,link -L . serde_derive.rs

lib%.rlib: %.rs
	$(RUSTC) --edition 2021 --crate-type=rlib --crate-name=$* --emit=metadata,link -L . $*.rs


