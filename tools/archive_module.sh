#!/usr/bin/env bash

EXPECTED_ARGS=3
E_BADARGS=65

if [[ $# -ne $EXPECTED_ARGS ]]
then
  echo "Usage: `basename $0` <kernel_dir> <module_dir> <module_name>"
  echo "Archive all the sources in <module_dir> as <module_name>"
  echo "Make sure to do this after a successful build"
  exit $E_BADARGS
fi

# Currently only "mm" causes problems if it is stripped this way
MODULE_SUPPORTED=0

KERNEL_DIR=$1
SUPPORTED_MODULES="drivers disk tty proc s5fs fs vm"
MODULE_DIR=$KERNEL_DIR/$2
MODULE_NAME=$3

for mod in $SUPPORTED_MODULES
do
    if [ $mod == $MODULE_NAME ]
    then
        MODULE_SUPPORTED=1
    fi
done

if [ $MODULE_SUPPORTED == 0 ]
then
    echo "Error: Unsupported module!"
    echo "Supported modules: $SUPPORTED_MODULES"
    exit 1
fi

if [ ! -d $MODULE_DIR ]
then
    echo Error: $MODULE_DIR does not exist!
    exit 1
fi

if [ ! -f $KERNEL_DIR/Makefile ]
then
    echo Error: $KERNEL_DIR/Makefile does not exist or is not a regular file!
    exit 1
fi

if [ -e $MODULE_DIR/lib$MODULE_NAME.a ]
then
    echo Error: $MODULE_DIR/lib$MODULE_NAME.a already exists!
    exit 1
fi

find $MODULE_DIR -type f -name "*.o" |
    xargs ar rcs $MODULE_DIR/lib$MODULE_NAME.a

sed -i "s#\(PREBUILT\s*:=\)#\1 $2/lib$MODULE_NAME.a#" $KERNEL_DIR/Makefile
