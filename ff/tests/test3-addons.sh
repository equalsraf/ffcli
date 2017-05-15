#!/bin/bash

. common.sh

ADDON_UBLOCK=$(mktemp --suffix=.xpi)
wget -O $ADDON_UBLOCK "https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/addon-607454-latest.xpi"
ADDON_COOKIES=$(mktemp --suffix=.xpi)
wget -O $ADDON_COOKIES "https://addons.mozilla.org/firefox/downloads/latest/self-destructing-cookies/addon-415846-latest.xpi"

export FF_PORT=$(ff start)
ff install $ADDON_UBLOCK
ff install $ADDON_COOKIES
