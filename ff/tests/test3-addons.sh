#!/bin/bash

. common.sh

ADDON_UBLOCK=$(mktemp --suffix=.xpi)
wget -O $ADDON_UBLOCK "https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/addon-607454-latest.xpi"

export FF_PORT=$(ff start)
ff install $ADDON_UBLOCK
