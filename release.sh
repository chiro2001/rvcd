#/bin/sh
if [[ -v BUILD ]]
then
  cargo build --release && \
  cp target/release/rvcd release/ && \
  upx release/rvcd && \
  cargo build --release --target=x86_64-pc-windows-gnu && \
  cp target/x86_64-pc-windows-gnu/release/rvcd.exe release/ && \
  upx release/rvcd.exe
fi
# trunk build --release && cp -r dist/ release/
rm -rf release.zip
if [[ -z DELETE_ASSETS ]]
then
  rm ../../scaleda/src/main/resources/bin/assets.zip
fi
# other asserts in release/ will also packed
cd release/ && 7z a ../release.zip -r * && cd ..
cd release/ && 7z a ../../scaleda/src/main/resources/bin/assets.zip -r * && cd ..
