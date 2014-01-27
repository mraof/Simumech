package com.mraof.simumech.irc;

import java.util.ArrayList;
import java.util.List;

public class IRC {

	public static List<IRCConnection> connections = new ArrayList<IRCConnection>();
	
	public IRC() 
	{
		connections.add(new IRCConnection("localhost"));
		connections.get(connections.size() - 1).channels.add("#test");
		(new Thread(connections.get(connections.size() - 1))).start();

//		connections.add(new IRCConnection("irc.esper.net"));
//		connections.get(connections.size() - 1).channels.add("#Kenbot");
//		connections.get(connections.size() - 1).channels.add("#bots");
//		connections.get(connections.size() - 1).socksProxy = "localhost";
//		connections.get(connections.size() - 1).socksPort = 8642;
//		(new Thread(connections.get(connections.size() - 1))).start();

		//        connections.add(new IRCConnection("irc.caffie.net"));
		//        connections.get(connections.size() - 1).channels.add("#zc");
		//        connections.get(connections.size() - 1).nick = "MraofMind";
		//        connections.get(connections.size() - 1).socksProxy = "localhost";
		//        connections.get(connections.size() - 1).socksPort = 8642;
		//        (new Thread(connections.get(connections.size() - 1))).start();
		
//		connections.add(new IRCConnection("chat.freenode.net"));
//		connections.get(connections.size() - 1).channels.add("#dreamvsdream");
//		connections.get(connections.size() - 1).socksProxy = "localhost";
//		connections.get(connections.size() - 1).socksPort = 8642;
//		(new Thread(connections.get(connections.size() - 1))).start();
	}
	
	public void command(String inputString)
	{
		
		boolean badSyntax = true;
		int splitIndex = inputString.indexOf(' ');
		if(splitIndex != -1)
		{
			String server = inputString.substring(0, splitIndex);
			inputString = inputString.substring(splitIndex + 1);
			splitIndex = inputString.indexOf(' ');
			if(splitIndex != -1)
			{
				String command = inputString.substring(0, splitIndex);
				inputString = inputString.substring(splitIndex + 1);
				boolean invalidConnection = true;
				for(IRCConnection connection : connections)
				{
					if(connection.hostname.equalsIgnoreCase(server))
					{
						connection.parser.onCommand("", "", command, inputString);
						invalidConnection = false;
						break;
					}
				}
				if(invalidConnection)
					System.out.println("Not connected to " + server);
			}
		}
	}
	public void quit()
	{
		for(IRCConnection connection : connections)
		{
			connection.running = false;
			connection.parser.add("");
		}
	}
}
