# Rorqual

Git repo analyser

# What?

Rorqual is a tool to sift through a git repository and generate a undirected graph from it. 
The files in the repository are represented by the nodes. 
The edges represent each time two files appeared in the same commit.

## Features

* Building a graph of all files that were committed together

* Fast-ish calculation of the betweennes centrality thanks to rayons parallelization

* Generating a .graphml file to analyze for example in Gephi

# Why?

I was inspired to look at code through the lens of graph theory by a Blog post that's sadly been deleted. The idea was to analyze a git repo with pythons networkx to find the files that are most coupled to the rest of all files. Those files then were good candidates as entry points to look at the code.
But it became clear that the python implementation was too memory hungry and i was looking for an excuse to use rust.
So I re-implemented the Brandes betweenness centrality algorithm from networkx in rust.
While playing around with this poc I discovered that a high degree of betweennes centrality is also an indicator of brittle tests. 
Another interesting possibility is to look at the generated graph with tools like Gephi to visualize
which files might be part of a module or feature and how closely this mirrors the intended architecture.
