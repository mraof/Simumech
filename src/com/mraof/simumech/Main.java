package com.mraof.simumech;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.Socket;
import java.net.UnknownHostException;

import com.mraof.simumech.network.IRCConnection;

public class Main 
{
	public static String clientName = "Simumech";
	public static String version = "0";
	
	public static void main(String args[])
	{
		if (args.length < 1) {
            System.err.println(
                "Usage: java EchoClient <host name> <port number>");
            System.exit(1);
        }
 
        String hostName = args[0];
        int portNumber = 6667;
        if(args.length >= 2)
        	portNumber = Integer.parseInt(args[1]);
 
        IRCConnection connection = new IRCConnection(hostName, portNumber);
        connection.channels.add("#test");
        (new Thread(connection)).start();
//        IRCConnection connection2 = new IRCConnection("irc.esper.net");
//        connection2.channels.add("#Kenbot");
//        connection2.channels.add("#bots");
//        connection2.socksProxy = "localhost";
//        connection2.socksPort = 8642;
//        (new Thread(connection2)).start();
//This code works, I'm just commenting it out to prevent channels from being flooded
	}
}
