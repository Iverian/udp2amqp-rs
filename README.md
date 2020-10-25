# udp2amqp

Utility for publishing datagrams to AMQP exchange.

## Configuration

Configuration is done through environment variables:

* `U2A_UDP_BIND_ADDR` (required): udp bind address in format `HOST:PORT`;
* `U2A_AMQP_URI` (required): AMQP broker URI;
* `U2A_AMQP_EXCHANGE` (default=`""`): AMQP exchange name;
* `U2A_AMQP_ROUTING_KEY` (default=`""`): AMQP routing key;
* `U2A_HTTP_PROBE_PORT` (default=`8080`): HTTP liveness probe port. Set to `0` to disable HTTP liveness probe server;
* `U2A_RECONNECT_DELAY_LIMIT_MS` (default=`60000`): maximum reconnect delay in milliseconds;
* `U2A_NO_RECONNECT` (default=`false`): Do not reconnect after connection failure;
* `U2A_DEBUG` (default=`false`): enable extensive logging;

## Testing

Directory [util](/util) contains Python3 scripts for simple Datagram produces and AMQP consumer.
