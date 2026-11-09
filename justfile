[windows]
set shell := ["C:\\Program Files\\Git\\bin\\sh.exe", "-c"]

# Fetch from remote origin using jj
pull:
    jj git fetch --remote origin

# Push to github
push-github:
    jj git push --tracked --remote origin

# Push to gitlab
push-gitlab:
    jj git push --tracked --remote gitlab

# Push to codeberg
push-codeberg:
    jj git push --tracked --remote codeberg

# Push to both github and gitlab
push: push-github push-codeberg

# Move changes to develop
move:
    jj b move --to '@-' develop

# Remove objects from older versions
tidy:
    cargo sweep --installed

# Install all binaries
install:
    cargo install --path cmd/convert-from-json
    cargo install --path cmd/find-airport
    cargo install --path acutectl
    cargo install --path process-data
