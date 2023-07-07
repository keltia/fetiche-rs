// Describe all different commands available
//

version = 1

cmds "convert" {
  type        = "filter"
  description = "Convert between the various formats into Cat21."
}

cmds "copy" {
  type        = "filter"
  description = "Just copy the data from the previous stage into the next one."
}

cmds "fetch" {
  type        = "producer"
  description = "Fetch a single piece of data from a Source."
}

cmds "message" {
  type        = "filter"
  description = "Insert a message in the pipeline."
}

cmds "nothing" {
  type        = "filter"
  description = "As the name implies, NOP."
}

cmds "read" {
  type        = "producer"
  description = "Read a block of data from a local file."
}

cmds "store" {
  type        = "consumer"
  description = "Split the incoming data into different files in a StorageArea."
}

cmds "stream" {
  type        = "producer"
  description = "Use a series of calls to generate a stream of data."
}

cmds "tee" {
  type        = "filter"
  description = "Like the tee(1) commands, save a copy of incoming data into a file."
}
