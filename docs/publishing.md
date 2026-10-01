# Initial publication

The local repository is ready to publish as `M-Umer-Farooq-Dev/actually-done`. No remote repository is created by preparing this checkout.

1. Create an empty public GitHub repository named `actually-done`. Do not initialize a second README, license, or gitignore.
2. From this checkout run:

```console
git remote add origin https://github.com/M-Umer-Farooq-Dev/actually-done.git
git push -u origin main
```

3. Run the checks in CONTRIBUTING.md on the publishing checkout. Review the README's platform limitations and dated benchmark before tagging a release.
4. When ready, create and push the version tag:

```console
git tag -a v0.1.0 -m "actually-done 0.1.0"
git push origin v0.1.0
```

Source installation is documented. Do not advertise prebuilt binaries or a crates.io package until those artifacts are actually published. A future binary release should include checksums and runtime validation for each advertised platform.
