from mcr.microsoft.com/windows/nanoserver:ltsc2022

# Install rust
RUN Invoke-WebRequest -Uri https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe -OutFile rustup-init.exe
RUN .\rustup-init.exe -y

# Install trunk
RUN cargo install --locked trunk

# Install nodejs
RUN Invoke-WebRequest -Uri https://nodejs.org/dist/v20.15.1/node-v20.15.1-x64.msi -OutFile node-v20.15.1-x64.msi
RUN ./node-v20.15.1-x64.msi /quiet
