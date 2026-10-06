First, we need a folder structure and a set of backlog items that implement the basic architecture for the small display, the controller service, and a hello_world service displayed on the hello_world format.

The hello_world service provides a greeting text and a quaternion attitude. The quaternion attitude represents the orientation of a virtual world that is tilted 23.5 degrees and which rotates about its axis once every twelve minutes.

The hellow_world format displays a box projection of the earth onto a sphere, oriented per the attitude from the hello_world service.

The sphere is lit from the upper right.

The small display initially has only one 'power' button in the console area.

The controller service initially sends DDS messages reprsenting values of a default xbox-controller layout. It cannot initially connect to a controller, but the service's window can manipulate the values of the controls via axis sliders and buttons.

