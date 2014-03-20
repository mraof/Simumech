package com.mraof.simumech.irc;

import java.util.ArrayList;
import java.util.List;

import com.mraof.simumech.IChat;

public class IRC implements IChat
{

	public static List<IRCConnection> connections = new ArrayList<IRCConnection>();
	public static String defaultNick = "Simumech";
	
	public IRC() 
	{
		connect("localhost", new String[]{"#test","#mraof"});
		connect("irc.esper.net", "megabnkalny", new String[]{"#Kenbot", "#bots"});
		
	}
	public void connect(String server)
	{
		connect(server, new String[]{});
	}
	public void connect(String server, String nick)
	{
		connect(server, nick, new String[]{});
	}
	private void connect(String server, String nick, String[] channels) 
	{
		connect(server, nick, channels, "", 0);
	}
	public void connect(String server, String[] channels)
	{
		connect(server, channels, "", 0);
	}
	public void connect(String server, String[] channels, String socksProxy, int socksPort)
	{
		connect(server, "", channels, socksProxy, socksPort);
	}
	public void connect(String server, String nick, String[] channels, String socksProxy, int socksPort)
	{
		IRCConnection connection = new IRCConnection(server);
		connection.socksProxy = socksProxy;
		connection.socksPort = socksPort;
		if(!nick.isEmpty())
			connection.nick = nick;
		for(String channel : channels)
			connection.channels.add(channel);
		(new Thread(connection)).start();
		connections.add(connection);
	}
	
	public void disconnect(String server)
	{
		for(IRCConnection connection : connections)
		{
			if(connection.hostname.equalsIgnoreCase(server))
			{
				connection.running = false;
				connection.parser.add("");
			}	
		}
	}
	
	public void command(String inputString)
	{
		
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
		else
		{
			if(inputString.equalsIgnoreCase("CONNECTIONS"))
			{
				String connectionsString = "";
				for(IRCConnection connection : connections)
					connectionsString += connection.hostname + " ";
				System.out.println(connectionsString);
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

	@Override
	public void message(String message) 
	{
		
	}
}
