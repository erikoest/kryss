# kryss

Crossword solver

## Usage

* Build code:
<pre>
cargo build release
</pre>

* Solve a crossword:
<pre>
./target/release/kryss examples/dagogtid-2017-12-22.kryss
&gt; help
</pre>

## Description

Kryss is a crossword solver with tty user interface. The solver takes
a description file as input argument:

<pre>
./target/release/kryss [--dictionary dict.json] mycrossword.kryss
</pre>

After starting up, kryss lookups up unknown keywords from the
norwegian crossword helper website https://gratiskryss.no. It then
tries to solve all the words which have exactly one candidate.

## Commands

### Solve

Solve all words which have exactly one candidate until all words have
either zero or multiple candidates.

### Board

Show board.

### Words

List words. The additional commands `placed` (solved words),
`unplaced` (unsolved words), `missing` (zero candidates), `ambiguous`
(multiple candidates) are available and will show only words with the
respectice conditions.

### Crossing &lt;key&gt;

For a given word, show all the crossing words.

### Candidates &lt;key&gt;

For a given word, show the known candidate words.

### Info &lt;key&gt;

For a given word, show miscellaneous information. This includes the
word position and orientation, crossing words and candidate list.

### Solution

Show the solution sentence.

### Place &lt;key&gt; &lt;word&gt;

Place a word into the crossword. The word is added to the dictionary
if it is not already known.

### lookup &lt;key&gt; [&lt;length&gt; | &lt;hint&gt;]

Lookup candidates for a keyword from the dictionary. The dictionary
consists of candidate words for all the known keywords. Candidates are
retrieved from the https://gratiskryssord.no website. The second
parameter is either numeric, giving the length of the word, or a
partially solved word on the form `..ab.c.` with dots representing an
unknown character.

### add &lt;key&gt; &lt;word&gt;

Add word to the dictionary.

### store board [&lt;filename&gt;]

Store the board. A filename may optionally be specified. The name of
the input description file is the default. The stored file has the
format of the description file, but with the placed words added to it.

### store dictionary [&lt;filename&gt;]

Store the dictionary file. A filename may optionally be specified. The
file `dict.json` is the default (this is also the dictionary file read
at startup).

### set colors [on|off]

Set tty colors on or off.

## Crossword description file

Each line in the desctiption file represents a word. The format is:

<pre>
O,X,Y,L,key[=word]
</pre>

Where:

* O is orientation R - right L - left D - down U - up X is horizontal
  * coordinate of the first character Y is vertical coordinate of the
  * first character L is length key is the hint word. If the word is
  * solved, the key is suffixed with an `=word` part.

The solution sentence has the form:

<pre>
S,O1,X1,Y1,L1,O2,X2,Y2,L2,...
</pre>

It represents a list of words, each of which does not have a
key. Together they form the solution sentence.
