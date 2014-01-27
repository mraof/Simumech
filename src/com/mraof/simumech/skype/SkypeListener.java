package com.mraof.simumech.skype;

import java.util.ArrayList;
import java.util.Random;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

import com.mraof.simumech.Main;
import com.skype.ChatMessage;
import com.skype.ChatMessageListener;
import com.skype.Skype;
import com.skype.SkypeException;

public class SkypeListener implements ChatMessageListener, Runnable
{
	public LinkedBlockingQueue<ChatMessage> messages = new LinkedBlockingQueue<ChatMessage>();
	//	public ArrayList<String> messageIds = new ArrayList<String>();
	SkypeBot parent;
	Random rand = new Random();
	ArrayList<String> ignored = new ArrayList<String>();

	public SkypeListener(SkypeBot parent) 
	{
		this.parent = parent;
		ignored.add("rubib-bot");
	}
	@Override
	public void run() 
	{
		while(parent.running)
		{
			try {
				ChatMessage message = messages.poll(30, TimeUnit.SECONDS);
				if(message == null || !message.getStatus().equals(ChatMessage.Status.RECEIVED))
				{
					//System.out.println("[Skype] Ignoring message because " + (message == null ? "it is null" : "the status is " + message.getStatus()));
					continue;
				}
				System.out.println("[Skype] " + message.getSenderDisplayName() + " (" + message.getSenderId() + "): " + message.getContent());
				double chance = rand.nextDouble();
				if(!ignored.contains(message.getSenderId()) && (message.getChat().getAllMembers().length <= 2 || (message.getContent().contains(Skype.getProfile().getFullName()))))
				{
					Main.markovChain.addLine(message.getContent());
					message.getChat().send(Main.markovChain.reply(message.getContent()));
				}
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
