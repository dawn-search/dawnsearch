# Architecture

Data flows like this:

    http <-> search <-> udp <-> internet

These steps happen when you perform a search through the web interface:

1. The http service receives your http request.
2. It performs a request on the search service.
3. The search service will search the local database, and contact the udp service to get results from the network.
4. The udp service will send the search out to the network.
5. Some time passes...
6. The udp service receives responses from the network, and after it has gathered enough it sends them back to the search system.
7. The search service combines the local and remote results and sends them to the http system.
8. The http service will generate a results page and send it back.

As you can see this is all message passing in the actor model. It's a bit convoluted, and a bit of a hassle to add new features,
but it is very reliable when the compiler is finally happy.
