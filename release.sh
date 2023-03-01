#/bin/sh
cargo build --release && \
cp target/release/rvcd release/ && \
upx release/rvcd && \
cargo build --release --target=x86_64-pc-windows-gnu && \
cp target/x86_64-pc-windows-gnu/release/rvcd.exe release/ && \
upx release/rvcd.exe
trunk build --release && cp -r dist/ release/
rm -rf release.zip && \
cd release/ && 7z a ../release.zip -r * && cd ..
cd release/ && 7z a ../../scaleda/src/main/resources/bin/assets.zip -r * && cd ..
