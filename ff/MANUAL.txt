
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

The __property__ command is similar to attr, however properties are JSON values. Strings are quoted, and null values are ignored.

	$ ff property html scrollWidth
	1907

## Windows/tabs

You can list the browser windows using the windows command, each line includes an id
and the title of the window

	$ ff windows
	8 "Google"
	14 "New Tab"

The id can be used with the switch command to switch windows.

