
**ff** is a CLI to command the firefox browser. You can get a list of commands
and options with the `help` command. Start the browser with:

```shelltest
$ ff --port 2929 start
```

You can open an URL with the `go` subcommand

```shelltest
$ ff --port 2929 go www.google.com
```

Finally you can close the browser with the `quit` command

```shelltest
$ ff --port 2929 quit
```
