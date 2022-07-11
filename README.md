<p align="center">
<br/>
<br/>
<br/>
   <img src="media/backpack-light.svg" width="300"/>
<br/>
<br/>
</p>
<p align="center">
<b>:white_check_mark: clone template projects easily</b>
<br/>
<b>:cowboy_hat_face: be lazy: <code>$ bp new acme/starter</code></b>
<br/>
<b>:robot: your own aliases <code>$ bp new rust-starter</code></b>
<br/>
<hr/>
</p>



# :school_satchel:	 Backpack <img src="https://github.com/rusty-ferris-club/backpack/actions/workflows/build.yml/badge.svg"/>

Use template and starter projects easily.

```
$ bp new user/repo
```

:white_check_mark: A supercharged scaffolding machine   
:white_check_mark: Grab subfolders, branches, tags from template projects    
:white_check_mark: Personalize shortlinks and aliases for individuals and teams  
:white_check_mark: Fast clone    
:white_check_mark: Apply files into current project  
:white_check_mark: No history or `.git` folder   
:white_check_mark: Local cache support   



# :rocket: Quick Start

Grab a release from [releases](https://github.com/jondot/bumblefoot/releases) and run help:
```
$ backpack --help
backpack 1.0.0
Create projects from existing repos

USAGE:
    backpack <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    apply     apply remote files into a folder [aliases: a]
    cache     cache handling
    config    create custom configuration
    help      Print this message or the help of the given subcommand(s)
    new       initialize a new project [aliases: n]
```

Put `bp` in an accessible bin folder:

```
mv ~/Downloads/bp ~/your-bins-folder/bp
```

Assuming `your-bins-folder` is in your `$PATH`, you should have this working:

```
$ which bp
/Users/user/your-bins-folder/bp
```

## :hammer: Using Backpack

The goal of `backpack` is to give you an easy tool for scaffolding [new projects](https://github.com/topics/template) from **existing repos**. It's optimized for laziness so you can specify a **shortlink** :).

### :link: What's a shortlink?

A shortlink is a pointer to a Git repo which looks like this:

![shortlink](media/shortlink.png)

Any one of these is a legal shortlink:

```
user/repo -> resolves to https://github.com/user/repo
gl:user/repo -> resolves to https://gitlab.org/user/repo
user/repo/-/subfolder -> takes only 'subfolder'
user/repo#wip -> takes the 'wip' branch
```

PS: You can customize the `gl:` prefix (or any other prefix) to resolve to what ever you want.
### :building_construction:	 Scaffolding new projects

Run:

```
$ bp new kriasoft/react-starter-kit my-react-project
```


:white_check_mark: Uses `new` to create **a new project**   
:white_check_mark: Resolves to [https://github.com/kriasoft/react-starter-kit](https://github.com/kriasoft/react-starter-kit)    
:white_check_mark: Finds the default branch, downloads it and caches locally. Next time you run, it'll be much faster.    

You can start with your new project:

```
$ cd my-react-project
$ git init .
$ yarn
```

### :arrow_right_hook:	 Applying templates to existing projects

Let's say you really like how `react-starter-kit` configured its Github Action, and you'd like to copy that to your **existing project**. You can do this:

```
$ bp apply kriasoft/react-starter-kit/-/.github
```


:white_check_mark: Use `/-/` to access a subfolder   
:white_check_mark: Use `apply` to overlay files onto your current working directory    

### :evergreen_tree:	 Using branches

Branches or tags can be used with the `#branch` specifier.


```
$ bp new kriasoft/react-starter-kit#feature/redux my-starter
```

### :woman_technologist: Using `git` for private repos

For **private repos**, you might want to download over Git SSH protocol, add `--git` to your commands:

```
$ bp new kriasoft/react-starter-kit --git
```
## :joystick:	Configuration

`backpack` is built for teams. This means you can configure your own shortcuts (called `aliases`) to Git hosting vendors, organizations, and repos.

### :raising_hand_woman:	 Configuring user aliases

If you have a template project you always use, you can give it a shortcut name, or an "alias".

Start by generating a **global user** configuration file:

```
$ bp config --init --global
```
And edit the file:

```
$ code ~/.backpack/backpack.yaml
```

To add aliases you can use the `aliases` section:

```yaml
aliases:
  rust-starter: 
    shortlink: rusty-ferris-club/rust-starter
```

And now you can use:

```
$ bp new rust-starter
```

Which will resolve to the correct location. Note: aliases will automatically resolve custom Git vendors (see below for what these are).

### :label:	 Configuring custom Git vendors

Start by generating a **project-local** configuration file:

```
$ bp config --init
wrote: .backpack.yaml.
```

Example: configure a Github Enterprise instance:

```yaml
vendors:
  custom:
    ghe: # <--- this prefix is yours
      kind: github
      base: enterprise-github.acme.org
             # `---- it will point here now
```

And now, you can use the `ghe:` prefix for your shortlinks:

```
$ bp new ghe:user/repo
```

You can check in the `.backpack.yaml` to your project to share it with your team. When `backpack` runs it will **pick it up automatically**.

You can also generate a **global user config** by specifying:

```
$ bp config --init --global
```



# Thanks

To all [Contributors](https://github.com/jondot/bumblefoot/graphs/contributors) - you make this happen, thanks!


# Copyright

Copyright (c) 2021 [@jondot](http://twitter.com/jondot). See [LICENSE](LICENSE.txt) for further details.
