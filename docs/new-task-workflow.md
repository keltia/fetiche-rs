

## Flow

job text -> job = Job::parse

        job -> run()
                create task list
                create key, stdout channel
                create pipeline from task list w/ fold()
                        [ task0(producer), task1(filter, .., taskN(filter), taskF(consumer)]
                for each task 
                        -> (stdin, _h) = call run(receiver)
                key.send()
                
        task -> run(receiver)
                        create stdout, stdin channel
                        data = receiver.recv()
                        execute(data, stdout)
                        (stdin, _h)






