#!/usr/bin/env python3
import random
from sys import stdout
from time import sleep
from  string import digits, ascii_letters

ASCII_CHARS = digits + ascii_letters
HANGUL_RANGE = (0xAC00, 0xD7B0)

def random_word (min_len=2, max_len=10):
  l = random.randrange (min_len, max_len+1)
  if random.random () < 0.7:
    return ''.join (random.choice (ASCII_CHARS) for _ in range (l))
  else:
    return ''.join (chr (random.randrange (*HANGUL_RANGE)) for _ in range (l))

def random_style ():
  if random.random () < 0.1:
    styles = [
      lambda: "\x1b[1m",
      lambda: "\x1b[2m",
      lambda: "\x1b[3m",
      lambda: "\x1b[{}m".format (random.randrange (30, 38)),
      lambda: "\x1b[{}m".format (random.randrange (90, 98)),
    ]
    return random.choice (styles) ()
  else:
    return ""

def main ():
  for i in range (1000):
    s = random_style ()
    if s:
      stdout.write (s)
    stdout.write (random_word ())
    if s:
      stdout.write ("\x1b[0m")
    if random.random () >= 0.9:
      stdout.write ('\n')
      sleep (0.1)
    else:
      stdout.write (' ')
    stdout.flush ()
  stdout.write ('\n')

if __name__ == "__main__":
  main ()

