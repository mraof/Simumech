package com.mraof.simumech;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.Socket;
import java.net.UnknownHostException;
import java.util.ArrayList;
import java.util.List;

import com.mraof.simumech.network.IRCConnection;

public class Main 
{
	public static String clientName = "Simumech";
	public static String version = "0";
	
	public static String[] owners = {"Mraof"};
	
	public static List<IRCConnection> connections = new ArrayList<IRCConnection>();
	
	public static void main(String args[])
	{
/*		if (args.length < 1) {
            System.err.println(
                "Usage: java EchoClient <host name> <port number>");
            System.exit(1);
        }*/
 
        String hostName = args[0];
        int portNumber = 6667;
        if(args.length >= 2)
        	portNumber = Integer.parseInt(args[1]);
 
        connections.add(new IRCConnection(hostName, portNumber));
        connections.get(connections.size() - 1).channels.add("#test");
        (new Thread(connections.get(connections.size() - 1))).start();
        
        
        BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(System.in));
        String inputString;
        
        try {
			while((inputString = bufferedReader.readLine()) != null)
			{
				if(inputString.toUpperCase().startsWith("MSG "))
				{
					boolean badSyntax = true;
					int splitIndex = inputString.indexOf(' ');
					if(splitIndex != -1)
					{
						inputString = inputString.substring(splitIndex + 1);
						splitIndex = inputString.indexOf(' ');
						if(splitIndex != -1)
						{
							String server = inputString.substring(0, splitIndex);
							inputString = inputString.substring(splitIndex + 1);
							splitIndex = inputString.indexOf(' ');
							if(splitIndex != -1)
							{
								String destination = inputString.substring(0, splitIndex);
								inputString = inputString.substring(splitIndex + 1);
								badSyntax = false;
								boolean invalidConnection = true;
								for(IRCConnection connection : connections)
								{
									if(connection.hostname.equalsIgnoreCase(server))
									{
										connection.parser.privmsg(destination, inputString);
										invalidConnection = false;
										break;
									}
								}
								if(invalidConnection)
									System.out.println("Not connected to " + server);
							}
						}
					}
					if(badSyntax)
						System.out.println("Format: MSG <server> <channel/nick> <message>");
				}
				if(inputString.equalsIgnoreCase("QUIT"))
				{
					for(IRCConnection connection : connections)
					{
						connection.running = false;
						connection.parser.add("");
					}
					break;
				}
			}
		} catch (IOException e) {
			e.printStackTrace();
		}
	}
}
