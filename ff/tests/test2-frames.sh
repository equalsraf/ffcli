#!/bin/bash

. common.sh

ff go file://$(pwd)/data/test2/frames.html
COUNT=$(ff exec -S "return document.location.href;" | wc -l)
test $COUNT = 5
