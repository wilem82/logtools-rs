# Summary

Log-processing tools.  Take log files in, process, output the results.

All tools operate on log entries, i.e. multiline messages.  A log entry's content is everything between the beginning of the log entry and the beginning of the next log entry.

Text matching is performed on the entirety of a log entry.

Tools take a `--help` argument.

# loggrep

Output filtered log entries.

Print messages that have `Exception` in them, but not `DEBUG`:
```
loggrep input.log -f Exception -F DEBUG
```

Store messages that have `WARN` and `ERROR` in them:
```
loggrep input.log -f WARN -f ERROR -o warn-error.log
```

# logmerge

Merge log files into a single stream of chronologically-ordered entries.  Works as long as the individual log files are already sorted. In rare cases when that's not the case, see `logsort` below.

Given this file hierarchy,
```
logs/
    host1/
        somelog.log
        junk.log
        junk.log.0
        junk.log.1
    host2/
        anotherlog.log
        morejunk.log
    host3/
        foo.log
        junk.log
```

- merges log entries of those files
- skipping the junk files
- prepending every output log entry with the source file it came from
- filters out everything that is not `WARN` or `ERROR`
```
logmerge logs -S -x {junk,morejunk}.log* | loggrep -L -o merged-filtered.log -f ERROR -f WARN
```

produces something like
```
host1/somelog.log: 2020-09-01 00:00:00.000 WARN - message1
host2/anotherlog.log: 2020-09-01 00:00:00.001 WARN - message1
host1/somelog.log: 2020-09-01 00:00:03.123 ERROR - message2
```

in `merged-filtered.log`.

# logsort

Takes a single log file and outputs its log entries in chronological order. Uses an external sort library, whose parameters as of now are hardcoded in `logsort` to use a temp directory and a 1MB buffer/temp file to perform the sort.

Useful in cases like access logs that have 1 record per handled request, even though there were two events - beginning of request and its end.  The timestamp in such logs might reflect the time of request's end, but quite often you're interested in when that request began (was received). If the time of request's beginning was also logged in some other field, `logsort` can be used to reorder the entries based on that deeper field.

Example (`input.log`):
```
2020-09-01 00:00:05.123 - GET http://localhost/foo - duration: 5123ms - status: 200 - start time: 01/09/2020 00:00:00.000
```

Can be handled with
```
logsort \
    input.log \
    -o input-reordered.log \
    --entry-pattern '^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} - .* start time: (?<timestamp>.+)$' \
    --timestamp-pattern '%d/%m/%Y %H:%M:%S.%3f'
```

arranging all entries with respect to the entries' start time.

# loguniq

Counts and shows distinct log entries.

- Only the first line of a log entry is processed
- Numbers are replaced with `<num>`

Far from being fully-featured.

# logoffset

Shifts the timestamp of all log messages by the specified amount of hours.

Far from being fully-featured.

# logplot

Generates an SVG chart for log entries per second. Typically one log entry corresponds to one network request or something.

Example:
```
logplot accesslog.log -o rps.svg -w 1300
```

As always, what a log entry is and how to parse its timestamp is defined via command line arguments.

Far from being fully-featured.
