# Automating Actions

Now that you're using Backpack as a fancy clone, you might be wanting to run some commands automatically such as:

* `yarn install`
* `make`
* `cargo build`
* `jest`

## Backpack configuration

You can start by having a _global configuration_ where you will state:

* A pointer to a repo
* The set of actions you want each time it is cloned


Run:

```
$ bp config --init
wrote: /Users/jondot/.backpack/backpack.yaml.
```

And configure a starter by editing the new `yaml` file:

```yaml
projects:
  vite-starter:
    shortlink: jvidalv/vital # you can use any custom prefix here too
    # is_git: true # force fetch from ssh
    actions:
      - name: run an install
        run: yarn install
```

Now you can run `bp` without any argument, and it will offer a selection:

```
$ bp
? Project (esc for shortlink) ›
❯ vite-starter (apply+new)
```

And you'll see `yarn` being run automatically:

```
$ bp
? Project (esc for shortlink) ›
❯ vite-starter (apply+new)
✔ Destination · my-project3
✔ Generate from 'vite-starter' into 'my-project3'? · Yes
🔮 Resolving...
🚚 Fetching content...
🎒 Unpacking files...
run an install
+ cd /Users/jondot/experiments
+ cd my-project3
+ yarn install
yarn install v1.22.17
[1/5] 🔍  Validating package.json...
[2/5] 🔍  Resolving packages...
[3/5] 🚚  Fetching packages...
```

## Self describing repos

If you fork `jvidalv/vital` to `your-user/vital`, you can add a local file called `.backpack-project.yml` with this:

```yaml
version: 1
new:
  vite-starter:
    shortlink: your-user/vital # you can use any custom prefix here too
    # is_git: true # force fetch from ssh
    actions:
      - name: run an install
        run: yarn install
```

Now every time you use `bp` to clone `your-user/vital` which contains the `.backpack-project.yml` file, the actions will run automatically!
