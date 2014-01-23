package com.mraof.simumech.network;

import java.util.concurrent.LinkedBlockingQueue;

public class MessageQueue implements Runnable
{
	public LinkedBlockingQueue<String> messages;
	IRCConnection connection;

	public MessageQueue(IRCConnection connection)
	{
		this.connection = connection;
		messages = new LinkedBlockingQueue<String>();
	}
	@Override
	public void run() 
	{
		while(this.connection.running)
		{
			try
			{
				String message = messages.take();
				if(!message.equals("~Goodnight~"))
				{
					connection.output.println(message);
					Thread.sleep((long) (Math.random() * 1000) + 500);
				}
			} catch(Exception e){e.printStackTrace();}
		}
		System.out.println("Stopping message queue for " + connection.hostname);
	}
	public void add(String message)
	{
		messages.add(message);
	}

}