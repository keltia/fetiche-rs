// Describe all different commands available
//

version = 1

cmds "convert" {
  type        = "Filter"
  description = "Convert between the various formats into Cat21."
}

cmds "copy" {
  type        = "Filter"
  description = "Just copy the data from the previous stage into the next one."
}

cmds "fetch" {
  type        = "Producer"
  description = "Fetch a single piece of data from a Source."
}

cmds "message" {
  type        = "Filter"
  description = "Insert a message in the pipeline."
}

cmds "nothing" {
  type        = "Filter"
  description = "As the name implies, NOP."
}

cmds "read" {
  type        = "Producer"
  description = "Read a block of data from a local file."
}

cmds "record" {
  type        = "Consumer"
  description = "Insert records into a database."
}

cmds "save" {
  type        = "Consumer"
  description = "Save into a single file, with possible a format change."
}

cmds "store" {
  type        = "Consumer"
  description = "Split the incoming data into different files in a StorageArea."
}

cmds "stream" {
  type        = "Producer"
  description = "Use a series of calls to generate a stream of data."
}

cmds "tee" {
  type        = "Filter"
  description = "Like the tee(1) commands, save a copy of incoming data into a file."
}
