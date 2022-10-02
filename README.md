# bowbend

## Conventional Commits

This project is using [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) to help automatically build change logs for each release.  Until we hit a certain level of maturity on this project, all changes are allowed to be breaking so ! will not be used yet.

The types used are:

- feat - This is used for all new features.
- fix - Bug fixes.
- docs - This is for expansion of documentation, either doc strings or markdown readmes
- ci - Improvements on CI, automated releases etc
- chore - This includes changes that don't fall into any of the above categories.  Things like bumping deps is a good example


The scopes used are:
- core - Changes for the core engine that could affect all SDKs and all scan types
- python - Changes that only impact the python SDK