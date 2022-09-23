# Replacing Content

While cloning a repo, and performing actions, you might want to personalize it or replace some string values. For this, you can use _Swaps_.

## Swapping and Rendering

Backpack contains a flexible and simple rendering engine, which will replace keywords that you define with given values through out the entire file tree, or for specific files you want.


The way to define those swaps, is again, in your configuration file:

```yaml
projects:
  my-project:
    shortlink: jvidalv/vital
    actions:
      - name: get input
        hook: before
        interaction:
          kind: input
          prompt: your name
          out: user_name
    swaps:
      - key: AUTHOR_NAME
        val_template: Dr. {{user_name}}
        path: src/.*
```

This will replace `AUTHOR_NAME`, if it exists, recursivly down the entire `src/` subtree.

## Taking user input

We've added _user input_ here, which is a new concept. An action can have an _interaction_, and some actions can be purely for interacting and taking input from a user.

```yaml
actions:
  - name: get input
    hook: before
    interaction:
      kind: input
      prompt: your name
      out: user_name
```

You can specify `before` or `after` for when the interaction appears. Then you can capture the input from the user into the `out` variable, which will be available in your _swaps_.

You can also offer selection menus:

```yaml
- name: select a DB
  interaction:
    kind: select
    prompt: select a database
    options:
      - sqlite
      - postgres
      - mysql
    default: sqlite
    out: db
```
