# Fetch from remote origin using jj
pull:
    jj git fetch --remote origin

# Push to github
push-github:
    jj git push --tracked --remote origin

# Push to gitlab
push-gitlab:
    jj git push --tracked --remote gitlab

# Push to both github and gitlab
push: push-github push-gitlab

# Move changes to develop
move:
    jj b move --to @- develop
