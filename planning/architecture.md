# To-Stations Prototype
The to-stations prototype is a set of distributed displays and services.
There are three configurations:
* rust
* c++
* hybrid

Each configuration has a launch script. They share a cleanup script. The launch script starts all of the displays and services for that configuration. The cleanup script stops all processes for all configurations.

Each configuration has two display applications. 
* Small display - Has a single square window that can display one 'format'. This window is surrounded by a console area that can display buttons and controls.
* Large display - Has a large rectangular windows that can display three square formats, with a narrow 'eyebrow' window at the top. This rectangular window is surrounded by a console area that can display buttons and controls.

Each application is in charge of rendering the console. It delegates rendering the window to a display library.

Display applications take a parameter on the command line that indicates their 'station number'.

Each configuration launches a large and a small display for stations 0 and 1.

Each station has a service that represents a joystick controller. For each of these services, a separate window application with a virtual controller will be launched. This window can also be bound to one game controller.

Each configuration launches a group of service applications that can run either headless or with a displayed console window.

It also launches a window that can start and stop the applications for that configuration.

All the applications communicate using Distributed Data Services configured to use IPv6 multicast on a Microsoft loopback adapter named StationsDDS.


