#/bin/sh
cargo build --release && \
cp target/release/rvcd release/ && \
upx release/rvcd && \
cargo build --release --target=x86_64-pc-windows-gnu && \
cp target/x86_64-pc-windows-gnu/release/rvcd.exe release/ && \
upx release/rvcd.exe
rm -rf release.zip
7z a release.zip -r release/
