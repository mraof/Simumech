package com.mraof.simumech.irc;

import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Calendar;
import java.util.concurrent.LinkedBlockingQueue;

import com.mraof.simumech.Main;
import com.mraof.simumech.Util;

public class MessageParser implements Runnable
{
	IRCConnection connection;
	MessageQueue queue;

	LinkedBlockingQueue<String> messages;

	public MessageParser(IRCConnection connection)
	{
		this.connection = connection;

		queue = connection.queue;
		messages = new LinkedBlockingQueue<String>();
	}
	public void onRecieved(String message)
	{
		if(message.isEmpty())
			return;
		int splitIndex = message.indexOf(' ');
		//String fullMessage = message;

		String source = "";
		if(message.charAt(0) == ':')
		{
			source = message.substring(1, splitIndex);
			message = message.substring(splitIndex + 1);
			splitIndex = message.indexOf(' ');
		}

		String type = message.substring(0, splitIndex);
		message = message.substring(splitIndex + 1);
		if(type.equalsIgnoreCase("PING"))
		{
			connection.output.println("PONG " + message);
			return;
		}

		//		println(fullMessage);

		splitIndex = message.indexOf(':');
		String parameters = "";
		if(splitIndex != -1)
		{
			if(splitIndex > 0)
				parameters = message.substring(0, splitIndex - 1); //remove space
			message = message.substring(splitIndex + 1);
		}

		switch(type.toUpperCase())
		{
			case "001":
				for(String channel : connection.channels)
					join(channel);
				return;
			case "433":
				println("Nick already in use, using " + connection.nick + "_");
				connection.nick = connection.nick + "_";
				connection.output.println("NICK " + connection.nick);
				return;
			case "PRIVMSG":
				onMessage(source, parameters, message);
				return;
			case "INVITE":
				join(message);
				println(connection.hostname + ": Invited to " + message);
				return;
			case "NICK":
				println(source + " is now known as " + message);
				return;
			default: 
				printf("Type: %s, source: %s, parameters: %s, message: %s\n", type, source, parameters, message);
		}

	}

	public void onMessage(String source, String destination, String message)
	{
		String sourceNick = source.substring(0, source.indexOf('!'));
		boolean pm = false;
		if(destination.equalsIgnoreCase(connection.nick))
		{
			pm = true;
			destination = sourceNick;
			if(destination.equalsIgnoreCase(connection.nick))
				return;
		}

		if(message.charAt(0) == '\u0001')
		{
			if(!onCTCP(source, destination, message.substring(1)))
				return;
			if(Util.splitFirst(message.substring(1))[0].equals("ACTION"))
			{
				printf("%s: * %s %s\n", destination, sourceNick, message.substring(7));
				return;
			}

		}
		printf("%s: <%s> %s\n", destination, sourceNick, message);

		if(message.startsWith(connection.prefix))
		{
			message = message.substring(connection.prefix.length());
			int splitIndex = message.indexOf(' ');
			String command = "";
			if(splitIndex == -1)
			{
				splitIndex = message.length() - 1;
				command = message;
				message = "";
			}
			else
			{
				command = message.substring(0, splitIndex);
				message = message.substring(splitIndex + 1);
			}


			try
			{
				if(onCommand(source, destination, command, message))
					return;
			} catch (Exception e) {e.printStackTrace(); return;}
		}

		//		println("PRIVMSG " + destination + " :" + message);
		if(pm || message.toLowerCase().contains(connection.nick.toLowerCase()))
			privmsg(destination, Main.markovChain.reply(message, connection.nick, sourceNick));
		if(!message.startsWith(connection.prefix))
			Main.markovChain.addLine(message);

	}
	public boolean onCTCP(String source, String destination, String message)
	{
		int end;
		if((end = message.indexOf('\u0001')) != -1)
			message = message.substring(0, end);
		int splitIndex = message.indexOf(' ');
		String type = "";
		if(splitIndex != -1)
		{
			type = message.substring(0, splitIndex);
			message = message.substring(splitIndex + 1);
		}
		else 
		{
			type = message;
			message = "";
		}

		String replyDestination = source.substring(0, source.indexOf('!'));

		printf("CTCP %s to %s from %s with message %s\n", type, destination, source, message);
		if(type.equalsIgnoreCase("PING"))
			ctcpReply(replyDestination, "PING", message);
		if(type.equalsIgnoreCase("VERSION"))
			ctcpReply(replyDestination, "VERSION", Main.clientName + ":" + Main.version + ":" + System.getProperty("os.name"));
		else if(type.equalsIgnoreCase("ACTION"))
			return true;
		else if(type.equalsIgnoreCase("TIME"))
			ctcpReply(replyDestination, "TIME", (new SimpleDateFormat()).format(Calendar.getInstance().getTime()));
		else if(type.equalsIgnoreCase("CLIENTINFO"))
		{
			if(message.isEmpty())
				ctcpReply(replyDestination, "CLIENTINFO", "PING VERSION ACTION TIME CLIENTINFO");
			else
			{
				String response = "";
				switch(message.toUpperCase())
				{
					case "PING":
						response = "PING <timestamp>";
						break;
					case "CLIENTINFO":
						response = "CLIENTINFO [command]";
						break;
					case "VERSION":
						response = "VERSION";
						break;
					case "ACTION":
						response = "ACTION <action message>";
						break;
					case "TIME":
						response = "TIME";
						break;
					default:
						response = "Unknown command";	
				}
				ctcpReply(replyDestination, "CLIENTINFO", response);
			}
		}

		return false;
	}
	public boolean onCommand(String source, String destination, String command, String message)
	{
		println("Recieved command \"" + command + "\" from \"" + source + "\"" + " in \"" + destination + "\"" + (message.isEmpty() ? " with arguments \"" + message + "\"" : ""));
		String userResponse = Main.userCommand(command + " " + message);
		if(!userResponse.isEmpty())
		{
			privmsg(destination, userResponse);
			return true;
		}
		boolean allowed = source.isEmpty();
		if(source.indexOf('!') != -1)
			for(String owner : Main.owners)
				if(source.substring(0, source.indexOf('!')).equals(owner))
				{
					allowed = true;
					break;
				}

		if(!allowed)
		{
			//println("User " + source + " attempted to use " + command.toUpperCase());
			return false;
		}
		String parts[];
		command = command.toUpperCase();
		switch(command)
		{
			case "QUIT":
				connection.running = false;
				break;
			case "RAW":
				connection.output.println(message);
				break;
			case "JOIN":
				connection.output.println("JOIN " + message);
				break;
			case "PART":
				if(message.isEmpty())
					message = destination;
				connection.output.println("PART " + message);
				break;
			case "EMPTY":
				queue.messages.clear();
				privmsg(destination, "Queue emptied");
				break;
			case "SAY":
				privmsg(destination, message);
				break;
			case "MSG":
				parts = Util.splitFirst(message);
				if(!parts[1].isEmpty())
				{
					destination = parts[0];
					message = parts[1];
				}
				else
					message = "Syntax: " + connection.prefix + "MSG <destination> <message>";
				privmsg(destination, message);
				break;
			case "NICK":
				connection.nick = message;
				connection.output.println("NICK " + connection.nick);
				break;
			case "CONNECT":
				String server = message;
				String socksProxy = "";
				int socksPort = 0;
				parts = Util.splitFirst(message);
				if(!parts[1].isEmpty())
				{
					server = parts[0];
					message = parts[1];
					parts = Util.splitFirst(message, ":");
					if(!parts[1].isEmpty())
					{
						socksProxy = parts[0];
						message = parts[1];
						try
						{
							socksPort = Integer.parseInt(message);
						}
						catch(NumberFormatException e){}
					}
				}
				IRC ircChat = (IRC) Main.chats.get("irc");
				if(socksPort == 0)
					ircChat.connect(server);
				else
					ircChat.connect(server, new String[]{}, socksProxy, socksPort);

				break;
			case "DISCONNECT":
				((IRC) Main.chats.get("irc")).disconnect(message);
				break;
			case "G":
				Main.globalCommand(message);
				break;
			case "SET":
				setFromString(message);
				break;
			default:
				return false;
		}
		return true;
	}


	public void privmsg(String destination, String message)
	{
		ArrayList<String> messages = Util.split(message, "\n");
		for(String currentMessage : messages)
		{
			println("[" + destination + "] Saying " + currentMessage);
			queue.add("PRIVMSG " + destination + " :" + currentMessage);
		}
	}
	public void notice(String destination, String message)
	{
		queue.add("NOTICE " + destination + " :" + message);
	}
	public void ctcp(String destination, String type, String message)
	{
		privmsg(destination, "\u0001" + type + (message.length() > 0 ? " " + message : "" ) + "\u0001");
	}
	public void ctcpReply(String destination, String type, String message)
	{
		notice(destination, "\u0001" + type + " " + message + "\u0001");
	}
	public void join(String channel)
	{
		connection.output.println("JOIN " + channel);
	}

	public void setFromString(String string)
	{
		String[] parts = Util.splitFirst(string);
		String varName = parts[0].toUpperCase();
		string = parts[1];
		switch(varName)
		{
			case "PREFIX":
				connection.prefix = string;
				break;
			case "NICK":
				IRC.defaultNick = string;
				break;
		}
	}
	@Override
	public void run() 
	{
		while(connection.running)
			try {
				onRecieved(messages.take());
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		queue.add("~Goodnight~");
		connection.output.println("QUIT :Quit message");
	}

	public void add(String message)
	{
		if(message != null)
			messages.add(message);
	}

	public void println(String string)
	{
		System.out.println("[IRC] [" + connection.hostname + "] " + string);
	}
	public void printf(String string, Object... parameters)
	{
		System.out.printf("[IRC] [" + connection.hostname + "] " + string, parameters);	
	}
}
