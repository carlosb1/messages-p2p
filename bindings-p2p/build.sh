cargo clean
rm -rf ../android-app/app/src/main/jniLibs

cargo build --release
# step 1
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 -o ../android-app/app/src/main/jniLibs build --release
# Step 2: Generate Kotlin bindings
cargo run --bin uniffi-bindgen generate src/bindings_p2p.udl \
    --language kotlin \
    --out-dir ../android-app/app/src/main/java

# Step 3: Copy the generated files to the correct location
cp ../android-app/app/src/main/jniLibs/x86_64/libbindings_p2p.so ../android-app/app/src/main/jniLibs/x86_64/libuniffi_bindings_p2p.so