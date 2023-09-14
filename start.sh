#!/bin/bash

# read from the file "redis.conf"
# the format is like
# [Server]
# [::]:45000, [::]:45001, [::]:45002
# [::]:46000, [::]:46001, [::]:46002

export PATH=$PATH:~/.cargo/bin

while read line
do
    if [[ $line =~ ^\[Server\] ]]; then
        cd mini-redis/
        while read line
        do
            if [[ $line =~ ^\[::\] ]]; then
                echo $line
                arr=(${line//,/ })
                for i in ${arr[@]}
                do
                    # split the ip address and port py the last ":"
                    host=${i%:*}
                    port=${i##*:}
                    args=("cargo" "run" "--bin" "server" $host $port)
                    # if $i is the first element of the array, add the address of the other servers
                    if [[ "$i" == "${arr[0]}" ]]; then
                        for j in ${arr[@]}
                        do
                            if [[ $j != $i ]]; then
                                port=${j##*:}
                                args+=("127.0.0.1:$port")
                            fi
                        done
                    fi
                    echo ${args[@]}
                    # run the server
                    ${args[@]} & > /dev/null
                done
            fi
        done
        cd ..
    fi
    continue
done<redis.conf