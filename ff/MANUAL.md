
ff is a CLI to command the firefox browser. You can get a list of commands
and options with the __help__ command.

	$ ff help

## Starting ff

ff can be started with the __start__ command
it in the background:

	$ ff start
	FF_PORT=36089

The output displays the port where the browser instance is listening for commands.
You can specify a specific port with the __port__ option

	$ ff start --port 2929

By default ff creates temporary profiles. If you want a persistent profile, provide
a path where the profile is to be saved

	$ ff start --profile test-profile

Note that firefox will refuse to run two instances for the same profile,
see see http://kb.mozillazine.org/Profile_in_use for more details.

To list available browsers managed by ff, use the __instances__ command. The
first part of the output is its listening port.

	$ ff instances
	36089/
	2929/

You can open an URL with the __go__ subcommand in the browser instance 2929

	$ ff go --port 2929 www.google.com

Finally you can close the browser with the __quit__ command

	$ ff quit --port 2929

To avoid using the port option in every command you can set the environment variable 
__$FF_PORT__ instead.

##  Getting page information

To get the source of the current wepage use the __source__ command

	$ ff go google.com
	$ ff source

Likewise for the __title__ and __url__

	$ ff title
	$ ff url

There are also commands to inspect web content. For example to print out all the text in paragraph tags, use the __text__ command

	$ ff text p

Empty elements are not printed.

The __attr__ command gets the value in a named html attribute, for example to get the href attribute for all anchors

	$ ff attr a href

The __property__ command is similar to attr, however properties are JSON values. Strings are quoted, and null values are not printed.

	$ ff property html scrollWidth
	1907

But the output can be filtered for type, e.g. to print only string properties without quotes

	$ ff property -S a href

## Executing Javascript

The __exec__ command is used run javascript code. The script will be executed in each frame, here is an example to list all frames in a page. null values are ignored.

	$ ff exec "return document.location.href;"

Arguments can be passed to the script, as additional positional arguments.

	$ ff exec "return arguments[0] + arguments[1];" 42 1
	43

Arguments are treated as JSON, passing a string may require double quoting the argument
according to your shell.

Scripts can also be passed from stdin

	$ echo "return 42;" | ff exec -
	42

You can execute async scripts, and force a maximum timeout of 10 seconds

	$ ff exec --async --timeout 10000 "setTimeout(function () { marionetteScriptFinished(42) }, 5000);"
	42

## Windows/tabs

You can list the browser windows using the windows command, each line includes an id
and the title of the window

	$ ff windows
	8 "Google"
	14 "New Tab"

The id can be used with the switch command to switch windows.

## Changing firefox preferences

The firefox preferences can be inspected with __prefget__

	$ ff prefget browser.uitour.enabled
	false

and modified with __prefset__. The last argument to __prefset__ is a json value, not a string.
This means passing strings requires quoting, and in some shells double quoting.
