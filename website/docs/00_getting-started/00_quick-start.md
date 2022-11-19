# Quick Start

Backpack can help you turn your own repos, or other people's repos, to a fully functioning starter project with all the boring stuff automated.




* **Generate from full project, subfolders, branches, tags :stars:** - use complete, versions, or any parts of repos you like
* **Shortcuts :rocket:** - create a personal or team  list of your projects with global and local shortcuts
* **Variable replacements** - replace variables in content and path (like cookiecutter)     
* **Automated setup steps :robot:** - run `yarn install` or `make` automatically after a clone
* **Interactive inputs** - define steps to take inputs and select options in YAML while generating a new project
*  **Fast & efficient :running:** - no history or `.git` folder, local caching of content by default, supporting `git` and `tar.gz` download

## Download

For macOS:

```
brew tap rusty-ferris-club/tap && brew install backpack
```

Otherwise, grab a release from [releases](https://github.com/rusty-ferris-club/backpack/releases) and run `bp --help`:

## Your first "clone"

Let's start with the `Vital` [vite.js](https://vitejs.dev/) template:

```bash
$ bp jvidalv/vital
✔ Destination · my-project2
✔ Generate from 'jvidalv/vital' into 'my-project2'? · Yes
🔮 Resolving...
🚚 Fetching content...
🎒 Unpacking files...

 + my-project2/.husky/.gitignore Copied
 + my-project2/.husky/commit-msg Copied
 + my-project2/.husky/pre-commit Copied
 + my-project2/index.html Copied
 + my-project2/tailwind.config.js Copied
 + my-project2/LICENSE Copied
 + my-project2/jest.config.js Copied
 + my-project2/.eslintrc Copied
 + my-project2/lint-staged.config.js Copied
 + my-project2/README.md Copied
   ... and 29 more file(s).

🎉 Done: 39 file(s) copied with 0 action(s).
```

In `my-project2` there's no git folder, and it was actually fetched from a Github tarball, which is faster.

Have an existing project and just want the cool `.husky` configuration from `Vital` to start with? do this:

```bash
$ cd your-existing-project
$ bp -f jvidalv/vital/-/.husky
✔ Destination ·
✔ Generate from 'jvidalv/vital/-/.husky' into 'a default folder'? · Yes
🔮 Resolving...
🚚 Fetching content...
🎒 Unpacking files...

 + .husky/.gitignore Copied
 + .husky/commit-msg Copied
 + .husky/pre-commit Copied

🎉 Done: 3 file(s) copied with 0 action(s).
```
