#!/bin/bash

# Run tests from inside the tests dir
pushd `dirname $0`
failed=()
for file in test*.sh; do
	if [ ! -x $file ]; then
		echo "$file is not executable"
		failed+=("$file")
		continue
	fi

	echo "##################### $file"
	./$file
	if [ "$?" != "0" ]; then
		failed+=("$file")
	fi
done

if [ -n "$failed" ]; then
  echo "Failed tests"
  for file in ${failed[@]}; do
    echo "- ${file}"
  done
  exit 1
fi

popd
