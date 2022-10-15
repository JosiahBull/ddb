#!/bin/bash

# download latest release from repo
curl -L https://github.com/josiahbull/dds/releases/download/v0.1.1/dds --output dds

# move binary to /usr/local/bin
sudo mv dds /usr/local/bin

# make executable
sudo chmod +x /usr/local/bin/dds

# run dds --generate <bash> where <bash> is the name of your shell, either zsh, fish, or bash

# if the user has .oh-my-zsh installed, run dds --generate zsh
if [ -d ~/.oh-my-zsh ]; then
    mkdir -p ~/.oh-my-zsh/completions/
    dds --generate zsh > ~/.oh-my-zsh/completions/_dds
    compinit
fi

echo "dds installed successfully, try running dds --help"
