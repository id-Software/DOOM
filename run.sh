#!/bin/bash


wad_file="DOOM.WAD"
expected_md5="c4fe9fd920207691a9f493668e0a2083"
if [[ ! -f "$wad_file" ]];then
  wget "https://archive.org/download/The_Ultimate_Doom/The_Ultimate_Doom.zip" 2>/dev/null;
  downloaded_zip="The_Ultimate_Doom.zip";
  if [[ ! -f "$downloaded_zip" ]];then
    echo "Could not find downloaded zip file.";
  fi
  unzipped_other=$(yes | unzip "$downloaded_zip" 2>/dev/null |grep -i 'inflating:' | cut -d : -f 2 | grep -v "$wad_file");
  rm -f "DOOM.EXE";
  download_md5=$(md5 -q "$wad_file");
  rm -f "$downloaded_zip";
  if [[ "$expected_md5" != "$download_md5" ]]; then
    rm -f "$wad_file";
    echo "Downloaded wad file does not match expected md5 hash";
  fi
fi

doom_exe="./build/doom";
if [[ ! -f "$doom_exe" ]];then
  yes | rm -f ./buid/*.o 2>/dev/null
  make 2>/dev/null
fi

if [[ ! -f "$doom_exe" ]];then
  echo "Could not find doom executable ${doom_exe}.";
else
  ${doom_exe} -config ./doom.config
fi

