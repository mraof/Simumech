package com.mraof.simumech.skype;

import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

import com.skype.ChatMessage;
import com.skype.ChatMessageListener;
import com.skype.SkypeException;

public class SkypeListener implements ChatMessageListener, Runnable
{
	public LinkedBlockingQueue<ChatMessage> messages = new LinkedBlockingQueue<ChatMessage>();
	SkypeBot parent;

	public SkypeListener(SkypeBot parent) 
	{
		this.parent = parent;
	}
	@Override
	public void run() 
	{
		while(parent.running)
		{
			try {
				ChatMessage message = messages.poll(30, TimeUnit.SECONDS);
				if(message == null)
					continue;
				System.out.println(message.getContent());
			} catch(InterruptedException e){Thread.currentThread().interrupt();}
			catch (SkypeException e) {e.printStackTrace();}
		}
	}

	@Override
	public void chatMessageReceived(ChatMessage receivedChatMessage) throws SkypeException 
	{
		if(receivedChatMessage != null)
			receivedChatMessage.getStatus();
			messages.add(receivedChatMessage);
	}

	@Override
	public void chatMessageSent(ChatMessage sentChatMessage) throws SkypeException {}

}
