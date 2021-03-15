#!/bin/bash
cat /dev/urandom | head -c200 | md5sum -b | cut -f1 -d' '
