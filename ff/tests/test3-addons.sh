#!/bin/bash

. common.sh

ADDON_UBLOCK=$(mktemp --suffix=.xpi)
wget -O $ADDON_UBLOCK "https://addons.mozilla.org/firefox/downloads/file/1580486/ublock_origin-1.18.2-an+fx.xpi"
ADDON_COOKIES=$(mktemp --suffix=.xpi)
wget -O $ADDON_COOKIES "https://addons.mozilla.org/firefox/downloads/file/832803/self_destructing_cookies-0.1.0-an+fx.xpi"

export FF_PORT=$(ff start)
ff install $ADDON_UBLOCK
ff install $ADDON_COOKIES
