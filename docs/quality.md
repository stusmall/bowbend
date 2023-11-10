
# Guide to Quality

This project has 5 main parts of its quality strategy.  It is best to view this strategy through the lens of the 
testing pyramid.  Each section of the testing strategy provides unique value.  No one section is "better" or "worse" 
than the other but just represent trade-offs in test speed, scope, granularity, reliability.

The tests at the base of the pyramid are the quickest to run, least flaky, and with the most narrow scope.  As we 
move up the pyramid, tests will cover more code and in a more realistic way.  The trade-off is that they will run 
much slower and the increased amount of moving parts means flakes are more likely.  Often tests are the base happen 
sooner in the development process and have a much shorter feedback loop to the developer.  Whenever possible we want 
to find bugs as close to the base as possible.

## Static Code Analysis

An easy to overlook base of the pyramid.  Using static code analysis we can find bug before even running the code.  
When we think of static code analysis we often think of external linting tools, but it also includes the language's 
native type system.  We want to lean on this whenever we can.  This is a large part of why Rust was used for this 
project.  It's powerful type system allows us to completely prevent entire classes of bugs, gives near-instant 
feedback during development without runtime costs.  Whenever possible we want to contain any business logic we can 
inside the Rust library `bowbend_core`.  This not allow prevents duplicating code across the different SDKs, it 
helps keep this code higher quality.

For languages that don't natively support typing we want to use 3rd party type checkers, like Python's mypy.  These 
tools often come with flaws and are less effective, we want all the help we can get.

In addition to the static code checking provided natively by the language we want to use as much 3rd party linting 
as possible and try and use some of the more pedantic settings.  These lints help make the code more idiomatic, more 
maintainable and can help prevent real bugs.

## Unit testing

Whenever possible we want to introduce small, mocked out unit tests over all appropriate code.  A unit test should 
never rely on an external resources, service or state.  It is crucial that unit tests remain reliable, fast and easy 
to run.  Another value of unit tests is their ability to inject faults or invalid data.  Since unit tests are 
designed to only cover smaller, (usually) internal APIs we are able to use data that would otherwise be difficult to 
cover in integration tests.

## Fuzzing

Today this isn't implemented but is needed.  We should liberally apply fuzzing around abstractions used for unit 
testing or for all rules.  Randomized, coverage guided testing is an extremely powerful tool for quickly finding 
strange bugs, crashes, security issues with very little human cost.


## Integration Tests

The vast majority of testing is handled via integration tests.  Docker is used to build a small network of containers to 
scan.  Originally we used Vagrant and VMs but GitHub started to block running VMs on all runner types, so it was 
switched to Docker. One big advantage of Vagrant is ease of configuration and the ability to configure ICMP per host.  
We have managed to hack together something similar with iptables, but it takes more effort.  Eventually if the CI 
solution changes we might switch back to a VM based solution.

Today we use real services to scan and identify but that won't scale.  We can't run a VM for each service we have a rule 
for and with a variety of different versions.  In the future we will introduce a mocking solution that will allow us 
to mock some aspects of arbitrary services.  

All integration tests are run twice.  We produce two artifacts for the SDK.  One is the release artifact.  It is 
important to validate exactly what is going out the door and into user's hands.  The other is an artifact with address 
sanitizer enabled.  This code base has a lot of unsafe blocks and asan is a powerful tool for ferreting out 
subtle undefined behavior.

## Manual Exploratory Testing

This is the most labor-intensive and happens the least often.  There is nothing that can replace a human kicking the 
tires, building some applications and scanning new services.  