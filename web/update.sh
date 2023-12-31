#!/bin/bash

ARTICLES_PATH="articles/"
ASSETS_PATH="assets/article/"

rm -rf $ARTICLES_PATH
git clone --depth 1 --branch master git@github.com:llenotre/blog-articles.git $ARTICLES_PATH
rm -rf "$ARTICLES_PATH/.git"

aws s3 sync s3://blog-assets $ASSETS_PATH