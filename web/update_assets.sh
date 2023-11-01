#!/bin/bash

CLONE_PATH="tmp"
ASSETS_PATH="assets/article/"

git clone https://github.com/llenotre/blog-assets.git $CLONE_PATH
mkdir -p $ASSETS_PATH
mv $CLONE_PATH/* $ASSETS_PATH
rm -rf $CLONE_PATH