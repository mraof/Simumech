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
		boolean isHandled = false;
		if(message.isEmpty())
			return;
		int splitIndex = message.indexOf(' ');
		String fullMessage = message;

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

		if(type.equals("001"))
		{
			for(String channel : connection.channels)
				join(channel);
			return;
		}
		if(type.equalsIgnoreCase("433"))
		{
			println("Nick already in use, using " + connection.nick + "_");
			connection.nick = connection.nick + "_";
			connection.output.println("NICK " + connection.nick);
			return;
		}

		splitIndex = message.indexOf(':');
		String parameters = "";
		if(splitIndex != -1)
		{
			if(splitIndex > 0)
				parameters = message.substring(0, splitIndex - 1); //remove space
			message = message.substring(splitIndex + 1);
		}

		if(type.equalsIgnoreCase("PRIVMSG"))
		{
			onMessage(source, parameters, message);
			return;
		}
		if(type.equalsIgnoreCase("INVITE"))
		{
			join(message);
			println(connection.hostname + ": Invited to " + message);
			isHandled = true;
		}
		if(type.equalsIgnoreCase("NICK"))
		{
			println(source + " is now known as " + message);
			isHandled = true;
		}

		if(!isHandled)
			printf("Type: %s, source: %s, parameters: %s, message: %s\n", type, source, parameters, message);
	}

	public void onMessage(String source, String destination, String message)
	{
		String sourceNick = source.substring(0, source.indexOf('!'));
		if(destination.equalsIgnoreCase(connection.nick))
		{
			destination = sourceNick;
			if(destination.equalsIgnoreCase(connection.nick))
				return;
		}

		if(message.charAt(0) == '\u0001')
		{
			if(!onCTCP(source, destination, message.substring(1)))
				return;
		}

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


			onCommand(source, destination, command, message);
			return;
		}

		//		println("PRIVMSG " + destination + " :" + message);
		if(message.toLowerCase().contains(connection.nick.toLowerCase()))
				privmsg(destination, Main.markovChain.reply(message, connection.nick, sourceNick));
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
	public void onCommand(String source, String destination, String command, String message)
	{
		println("Recieved command \"" + command + "\" from \"" + source + "\"" + " in \"" + destination + "\"" + (message.isEmpty() ? " with arguments \"" + message + "\"" : ""));
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
			println("User " + source + " attempted to use " + command.toUpperCase());
			return;
		}

		if(command.equalsIgnoreCase("QUIT"))
			connection.running = false;
		else if(command.equalsIgnoreCase("RAW") && !message.isEmpty())
			connection.output.println(message);
		else if(command.equalsIgnoreCase("JOIN") && !message.isEmpty())
			connection.output.println("JOIN " + message);
		else if(command.equalsIgnoreCase("PART"))
		{
			if(message.isEmpty())
				message = destination;
			connection.output.println("PART " + message);
		}
		else if(command.equalsIgnoreCase("EMPTY"))
		{
			queue.messages.clear();
			privmsg(destination, "Queue emptied");
		}
		else if(command.equalsIgnoreCase("SAY"))
		{
			privmsg(destination, message);
		}
		else if(command.equalsIgnoreCase("MSG"))
		{
			int splitIndex = message.indexOf(' ');
			if(splitIndex != -1)
			{
				destination = message.substring(0, splitIndex);
				message = message.substring(splitIndex + 1);
			}
			else
				message = "Syntax: " + connection.prefix + "MSG <destination> <message>";
			privmsg(destination, message);
		}
		else if(command.equalsIgnoreCase("NICK"))
		{
			connection.nick = message;
			connection.output.println("NICK " + connection.nick);
		}
		else if(command.equalsIgnoreCase("CONNECT"))
		{
			String server = message;
			String socksProxy = "";
			int socksPort = 0;
			int splitIndex = message.indexOf(' ');
			if(splitIndex != -1)
			{
				server = message.substring(0, splitIndex);
				message = message.substring(splitIndex + 1);
				splitIndex = message.indexOf(' ');
				if(splitIndex != -1)
				{
					socksProxy = message.substring(0, splitIndex);
					message = message.substring(splitIndex + 1);
					try
					{
						socksPort = Integer.parseInt(message);
					}
					catch(NumberFormatException e){}
				}
				IRC ircChat = (IRC) Main.chats.get("irc");
				if(socksPort == 0)
					ircChat.connect(server);
				else
					ircChat.connect(server, new String[]{}, socksProxy, socksPort);
			}
		}
		else if(command.equalsIgnoreCase("DISCONNECT"))
			((IRC) Main.chats.get("irc")).disconnect(message);
		else if(command.equalsIgnoreCase("G"))
				Main.globalCommand(message);
		else if(command.equalsIgnoreCase("M"))
		{
			String output = Main.markovChain.command(message);
			if(!output.isEmpty())
				privmsg(destination, output);
		}
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
	public void printf(String string, Object... paramaters)
	{
		
	}
}
