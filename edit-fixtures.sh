#! /usr/bin/env bash

if [[ -z $1 ]]; then
	echo -e "\e[31m[ ERR ] no fixture name given"
	echo -e "Possibilities:"
	find tests -iname .gitted | sed 's!^.*/\([^/]*\)/.gitted!\1!' 1>&2
	exit 1
fi
if ! [[ -d ./tests/fixtures/$1 ]]; then
	echo -e "\e[31m[ ERR ] ./tests/fixtures/$1 not found"
	exit 1
fi

tmp_dir=$(mktemp -d -t "$1-XXXXX")

mv "./tests/fixtures/$1" $tmp_dir

old_pwd=$PWD
cd $tmp_dir/$1

if [[ -d .gitted ]]; then 
	mv .gitted  .git
fi
if [[ -f gitattributes ]]; then
	mv gitattributes .gitattributes
fi
if [[ -f gitignore ]]; then
	mv gitignore .gitignore
fi

eval $SHELL 

if [[ -d .git ]]; then 
	mv .git .gitted
fi
if [[ -f .gitattributes ]]; then
	mv .gitattributes gitattributes
fi
if [[ -f .gitignore ]]; then
	mv .gitignore gitignore
fi

cd $old_pwd

mv "$tmp_dir/$1" "./tests/fixtures/$1"
