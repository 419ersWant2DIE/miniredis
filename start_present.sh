#!/bin/bash

# read from the file "redis.conf"
# the format is like
# [Server]
# [::]:45000, [::]:45001, [::]:45002
# [::]:46000, [::]:46001, [::]:46002
# [Proxy]
# [::]:41000

export PATH=$PATH:~/.cargo/bin

CONF="redis.conf"
REDIS="mini-redis"
PROXY="redis_proxy"
DEFAULT_HOST="127.0.0.1"
BUILD="cargo build"
BIN="cargo run --bin server"
SERVERS=""

while read line
do
    if [[ $line =~ ^# ]]; then
        continue
    fi
    if [[ $line =~ ^\[Server\] ]]; then
        cd $REDIS
        $BUILD
        while read line
        do
            if [[ $line =~ ^# ]]; then
                continue
            fi
            if [[ $line =~ ^\[Proxy\] ]]; then
                break
            fi

            # split the line by ","
            arr=(${line//,/ })

            # add the "-n" flag
            SERVERS+=" -n"

            for i in ${arr[@]}
            do
                # split the ip address and port py the last ":"
                host=${i%:*}
                port=${i##*:}
                args=($BIN $host $port)
                SERVERS+=" $DEFAULT_HOST:$port"
                # if $i is the first element of the array, add the address of the other servers
                if [[ "$i" == "${arr[0]}" ]]; then
                    for j in ${arr[@]}
                    do
                        if [[ "$j" != "$i" ]]; then
                            host=$DEFAULT_HOST
                            port=${j##*:}
                            args+=("$host:$port")
                        fi
                    done
                fi
                # run the server
                ${args[@]} &
            done
        done
        cd ..
    fi
    if [[ $line =~ ^\[Proxy\] ]]; then
        cd $PROXY
        $BUILD
        while read line
        do
            if [[ $line =~ ^# ]]; then
                continue
            fi
            # split the ip address and port py the last ":"
            host=${line%:*}
            port=${line##*:}
            args=("$BIN $host:$port $SERVERS")
            # run the server
            ${args[@]}
        done
        cd ..
    fi
    continue
done < $CONF