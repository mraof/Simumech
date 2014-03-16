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
	String owners[] = {"mraof.null"};

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
			//double chance = rand.nextDouble();
	
			boolean commanded = false;
			if(message.getContent().startsWith(commandPrefix) || message.getContent().toUpperCase().startsWith(Skype.getProfile().getFullName().toUpperCase()))
				commanded = onCommand(message);
			if(!ignored.contains(message.getSenderId()) && !commanded && (message.getChat().getAllMembers().length <= 2 || (message.getContent().toUpperCase().contains(Skype.getProfile().getFullName().toUpperCase()))))
			{
				message.getChat().send(Main.markovChain.reply(message.getContent(), Skype.getProfile().getFullName(), message.getSenderDisplayName()));
			}
		} catch (SkypeException e) {e.printStackTrace();}
	}
	public boolean onCommand(ChatMessage chatMessage)
	{
		try {
			System.out.println("[Skype] Processing command " + chatMessage.getContent() + " from " + chatMessage.getSenderId());
			int splitIndex = commandPrefix.length();
			if(!chatMessage.getContent().startsWith(commandPrefix))
				splitIndex = chatMessage.getContent().indexOf(' ') + 1;
			if(splitIndex == 0)
				return false;
			String message = chatMessage.getContent().substring(splitIndex);
			String response = Main.userCommand(message);
			if(!response.isEmpty())
			{
				chatMessage.getChat().send(response);
				return true;
			}
			boolean allowed = false;
			for(String owner : owners)
				if(chatMessage.getSenderId().equals(owner))
					allowed = true;
			if(!allowed)
				return false;
			splitIndex = message.indexOf(' ');
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
				response = Main.globalCommand(message);
				if(!response.isEmpty())
					chatMessage.getChat().send(response);
			}
			else if(command.equalsIgnoreCase("M"))
			{
				response = Main.markovChain.command(message);
				if(!response.isEmpty())
					chatMessage.getChat().send(response);
			}
			else
				return false;
			return true;
		} catch (SkypeException e) {
			e.printStackTrace();
		}
		return false;

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
