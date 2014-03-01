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
	String commandPrefix = "$";

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
				ChatMessage message = messages.poll(10, TimeUnit.SECONDS);
				if(message != null)
				{
					if(!message.getSender().getId().equals(Skype.getProfile().getId()))
						Main.markovChain.addLine(message.getContent());
					if(message.getStatus().equals(ChatMessage.Status.RECEIVED))
						onMessage(message);
				}

			} catch(InterruptedException e){Thread.currentThread().interrupt();break;}
			catch (SkypeException e) {e.printStackTrace();}
		}
	}

	public void onMessage(ChatMessage message)
	{
		try {
			System.out.println("[Skype] " + message.getSenderDisplayName() + " (" + message.getSenderId() + "): " + message.getContent());
			double chance = rand.nextDouble();

			if(message.getContent().startsWith(commandPrefix))
				onCommand(message);
			else if(!ignored.contains(message.getSenderId()) && (message.getChat().getAllMembers().length <= 2 || (message.getContent().toUpperCase().contains(Skype.getProfile().getFullName().toUpperCase()))))
			{
				message.getChat().send(Main.markovChain.reply(message.getContent(), Skype.getProfile().getFullName(), message.getSenderDisplayName()));
			}
		} catch (SkypeException e) {e.printStackTrace();}
	}
	public void onCommand(ChatMessage chatMessage)
	{
		try {
			String message = chatMessage.getContent().substring(commandPrefix.length());
			int splitIndex = message.indexOf(' ');
			String command;
			if(splitIndex != -1)
			{
				command = message.substring(0, splitIndex);
				message = message.substring(splitIndex + 1);
			}
			else 
			{
				command = message;
				message = "";
			}

			if(command.equalsIgnoreCase("SAY"))
				chatMessage.getChat().send(message);
			else if(command.equalsIgnoreCase("G"))
			{
				String response = Main.globalCommand(message);
				if(!response.isEmpty())
					chatMessage.getChat().send(response);
			}
			else if(command.equalsIgnoreCase("M"))
			{
				String response = Main.markovChain.command(message);
				if(!response.isEmpty())
					chatMessage.getChat().send(response);
			}
		} catch (SkypeException e) {
			e.printStackTrace();
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
