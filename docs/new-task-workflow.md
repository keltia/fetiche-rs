
## Actors

- supervisor        father of all, register events
  - scheduler       manage job executions, queues, etc.
  - runner          get job from queue, and execute it
  - sources         load sites
  - state           register state changes
  - stats           stats for jobs
  - storage (WIP)   manage storage (files, directories)
  - tokens (WIP)    manage tokens lifecycle


Right now:

  scheduler
    every tick
      check waiting queue
      if something
        launch factory runner
      else
        loop
          

## State machine

create_job
  None -> Created
    Job::site() / Job::with() -> Ready
parse_job
  None -> Ready
submit_job
  Ready ->
    queue_job
      -> Queued
     Dispatch -> worker

every TICK
runner -> get next job

We have 3 queues:

- waiting
- running
- finished

## Flow

job text -> job = Job::parse

Job -> queue

queue -> Job into runner

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






