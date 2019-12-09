#! /usr/bin/env sh

../../target/release/process-with-lua headers \
	-i ../../pages-articles.xml \
	-s language_headers.lua \
	-n main -n reconstruction -n appendix